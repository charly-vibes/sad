//! Purpose: decide which files a gate run analyses, on evidence taken from
//! commit history rather than from names, paths, or working-tree guesses.
//! Responsibilities: parse git trailer blocks (final paragraph only, exact
//! key match after the first separator), resolve the last commit that touched
//! a repo-relative path, and select files for a default (attributed-only) run
//! or a full `--all` run.
//! Rationale: default scope is the adoption guard — it must analyze new agent
//! work and skip untouched legacy prose. No-inference rule: attribution comes
//! only from the explicit commit trailer configured as `attribution.trailer`;
//! author names, file names, directory layout, and working-tree state are
//! never treated as evidence. Every git or decoding failure is non-fatal and
//! deterministically degrades to "not attributed" (or "no governing commit"
//! for a path git has never seen) instead of panicking.
//!
//! Policies:
//! - Pathspecs are always literal (`--literal-pathspecs`): a repo-relative
//!   path such as `app/blog/[slug]/page.tsx` is looked up as that exact file,
//!   never expanded as a git glob that could match a sibling like
//!   `app/blog/s/page.tsx`.
//! - A shallow-clone boundary commit has no parents and therefore no history
//!   that git can consult; its message is treated as having no trustworthy
//!   trailer, so default mode excludes it. Shallow repositories are not
//!   unshallowed (that would need the network).
//! - Commit messages are decoded lossily: invalid UTF-8 must not silently
//!   hide a trailer. Only a git *failure* excludes a file.

use std::path::Path;
use std::process::Command;

use crate::config::Config;

/// Trailer prefixes git itself generates and therefore recognises even when
/// the surrounding paragraph also holds non-trailer lines.
const GIT_GENERATED_TRAILER_PREFIXES: [&str; 2] =
    ["Signed-off-by: ", "(cherry picked from commit "];

/// True when the trailer block of `message` asserts `trailer`.
///
/// Follows git's trailer rules: the first paragraph is the subject and can
/// never be a trailer block; only the last paragraph is considered; keys are
/// compared case-insensitively and must match exactly, so `Not-Assisted-by`
/// is not `Assisted-by`. An empty message has no trailer.
pub fn has_agent_trailer(message: &str, trailer: &str) -> bool {
    if trailer.is_empty() {
        return false;
    }

    let lines: Vec<&str> = message.lines().collect();
    // The first paragraph is the title; without a blank line there is no
    // later paragraph that could be a trailer block.
    let Some(body_start) = lines.iter().position(|line| is_blank(line)) else {
        return false;
    };

    let mut recognized_prefix = false;
    let mut trailer_lines: usize = 0;
    let mut non_trailer_lines: usize = 0;
    let mut continuation_lines: usize = 0;
    let mut matching_trailer = false;
    let mut only_blank_so_far = true;

    // Walk the last paragraph backwards. The first blank line after content
    // closes the block, exactly as git delimits it.
    for &line in lines[body_start..].iter().rev() {
        if is_comment(line) {
            non_trailer_lines += continuation_lines;
            continuation_lines = 0;
            continue;
        }
        if is_blank(line) {
            if only_blank_so_far {
                continue;
            }
            non_trailer_lines += continuation_lines;
            return trailer_block_closes(recognized_prefix, trailer_lines, non_trailer_lines)
                && matching_trailer;
        }
        only_blank_so_far = false;

        let key = trailer_key(line);
        if is_generated_trailer(line) {
            trailer_lines += 1;
            continuation_lines = 0;
            recognized_prefix = true;
        } else if key.is_some() {
            trailer_lines += 1;
            continuation_lines = 0;
        } else if line.starts_with(' ') || line.starts_with('\t') {
            continuation_lines += 1;
        } else {
            non_trailer_lines += 1;
            non_trailer_lines += continuation_lines;
            continuation_lines = 0;
        }

        if let Some(key) = key
            && key.eq_ignore_ascii_case(trailer)
        {
            matching_trailer = true;
        }
    }

    false
}

/// The full hash of the last commit that touched `rel_path`, or `None` when
/// git has never recorded that path or any git invocation fails.
///
/// `rel_path` is repo-relative and uses forward slashes; a failed lookup is
/// reported as "no governing commit" so callers degrade instead of panicking.
pub fn governing_revision(repo: &Path, rel_path: &str) -> Option<String> {
    let output = git_command(repo)
        .args(["log", "-1", "--no-show-signature", "--format=%H", "--"])
        .arg(rel_path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let revision = std::str::from_utf8(&output.stdout).ok()?.trim();
    if revision.is_empty() {
        None
    } else {
        Some(revision.to_string())
    }
}

/// Filter `files` down to the ones a gate run should analyse.
///
/// With `all`, every input file is returned in input order. Otherwise a file
/// is kept when git has no governing commit for it (new or untracked work) or
/// when that commit's full message carries the configured trailer. Order is
/// preserved and attribution is never inferred from anything but the trailer.
pub fn select(repo: &Path, files: &[String], cfg: &Config, all: bool) -> Vec<String> {
    if all {
        return files.to_vec();
    }

    let trailer = cfg.attribution.trailer.as_str();
    files
        .iter()
        .filter(|file| match governing_revision(repo, file) {
            None => true,
            Some(revision) => governing_commit_has_trailer(repo, &revision, trailer),
        })
        .cloned()
        .collect()
}

/// Whether the full message of `revision` carries `trailer`; any failure to
/// read the commit is reported as "not attributed".
///
/// A shallow-clone boundary commit is excluded before its message is read:
/// the clone cut history off there, so git cannot tell whether that message
/// is the real origin of the path. Default mode stays conservative and treats
/// it as unattributed; `select(..., all = true)` still includes the file.
fn governing_commit_has_trailer(repo: &Path, revision: &str, trailer: &str) -> bool {
    if is_shallow_boundary(repo, revision) {
        return false;
    }

    let output = match git_command(repo)
        .args(["log", "-1", "--no-show-signature", "--format=%B"])
        .arg(revision)
        .output()
    {
        Ok(output) => output,
        Err(_) => return false,
    };
    if !output.status.success() {
        return false;
    }
    // Lossy decoding: a non-UTF-8 message must still reach trailer detection.
    // Trailer keys are ASCII, so replacement characters cannot fabricate one.
    let message = String::from_utf8_lossy(&output.stdout);
    has_agent_trailer(&message, trailer)
}

/// True when the repository is shallow and `revision` is its history boundary.
///
/// In a shallow clone the boundary commit is presented as parentless, so
/// `rev-list --parents -n 1` printing only the commit itself marks the cut
/// point. Repositories git cannot confirm as shallow (including versions
/// without `--is-shallow-repository`) report `false`, which degrades to the
/// ordinary message check.
fn is_shallow_boundary(repo: &Path, revision: &str) -> bool {
    let is_shallow = git_command(repo)
        .args(["rev-parse", "--is-shallow-repository"])
        .output()
        .is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true"
        });
    if !is_shallow {
        return false;
    }

    git_command(repo)
        .args(["rev-list", "--parents", "-n", "1"])
        .arg(revision)
        .output()
        .is_ok_and(|output| {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout)
                    .split_whitespace()
                    .count()
                    < 2
        })
}

/// A git command rooted at `repo` with inherited repository overrides removed
/// so the requested path always wins, and pathspec globbing disabled so a
/// repo-relative path is always interpreted as the literal file name.
fn git_command(repo: &Path) -> Command {
    let mut command = Command::new("git");
    command.args(["--literal-pathspecs", "-C"]).arg(repo);
    command
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    command
}

/// The key of a git trailer line: an alphanumeric/hyphen token (optionally
/// followed by whitespace) before the first `:` separator.
fn trailer_key(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    if bytes.first()?.is_ascii_whitespace() {
        return None;
    }

    let mut whitespace_found = false;
    for (index, &byte) in bytes.iter().enumerate() {
        if byte == b':' {
            return (index >= 1).then(|| line[..index].trim());
        }
        if !whitespace_found && (byte.is_ascii_alphanumeric() || byte == b'-') {
            continue;
        }
        if index != 0 && (byte == b' ' || byte == b'\t') {
            whitespace_found = true;
            continue;
        }
        return None;
    }
    None
}

/// A trailer line git generates itself, recognised regardless of config.
fn is_generated_trailer(line: &str) -> bool {
    GIT_GENERATED_TRAILER_PREFIXES
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

/// Git's closing rule for the last paragraph: all trailer lines, or at least
/// one recognised prefix and trailers making up at least a quarter of it.
fn trailer_block_closes(
    recognized_prefix: bool,
    trailer_lines: usize,
    non_trailer_lines: usize,
) -> bool {
    if trailer_lines == 0 {
        return false;
    }
    if non_trailer_lines == 0 {
        return true;
    }
    recognized_prefix && trailer_lines.saturating_mul(3) >= non_trailer_lines
}

/// True when the line holds only whitespace.
fn is_blank(line: &str) -> bool {
    line.bytes().all(|byte| byte.is_ascii_whitespace())
}

/// True when the line is a comment, which git ignores while scanning trailers.
fn is_comment(line: &str) -> bool {
    line.starts_with('#')
}
