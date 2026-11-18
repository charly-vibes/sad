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
//! - Repository-level facts are resolved once per selection: the shallow
//!   probe runs once and each revision's trailer verdict is memoised, so a
//!   commit shared by many files is never re-read.
//! - Selection reports how each candidate was classified (attributed, no
//!   governing commit, unattributed) so an empty scope is explainable rather
//!   than indistinguishable from a clean tree; the counters never influence
//!   which files are kept.

use std::collections::BTreeMap;
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

    trailer_block(&lines[body_start..], trailer)
}

/// One commit-message line, classified for the backwards trailer scan.
struct TrailerLine<'a> {
    /// The line's trailer key, when it has one.
    key: Option<&'a str>,
    /// The line is a trailer git generates itself.
    generated: bool,
    /// The line continues the trailer above it.
    continuation: bool,
}

/// Running totals of a backwards scan over the last paragraph.
#[derive(Default)]
struct TrailerScan {
    recognized_prefix: bool,
    trailer_lines: usize,
    non_trailer_lines: usize,
    continuation_lines: usize,
    matching_trailer: bool,
    /// Whether the scan has reached a content line of the paragraph.
    seen_content: bool,
}

/// Walk `body` (the message from its first blank line on) backwards and report
/// whether it is a trailer block carrying `trailer`.
///
/// The first blank line after content closes the block, exactly as git
/// delimits it; comments are skipped the same way.
fn trailer_block(body: &[&str], trailer: &str) -> bool {
    let mut scan = TrailerScan::default();

    for &line in body.iter().rev() {
        if is_comment(line) {
            scan.flush_continuation();
            continue;
        }
        if is_blank(line) {
            if !scan.seen_content {
                continue;
            }
            scan.flush_continuation();
            let closes = trailer_block_closes(
                scan.recognized_prefix,
                scan.trailer_lines,
                scan.non_trailer_lines,
            );
            return closes && scan.matching_trailer;
        }
        scan.seen_content = true;
        scan.absorb(classify_trailer_line(line), trailer);
    }

    false
}

/// Classify a single message line by what it contributes to the trailer scan.
fn classify_trailer_line(line: &str) -> TrailerLine<'_> {
    let key = trailer_key(line);
    let generated = is_generated_trailer(line);
    TrailerLine {
        key,
        generated,
        continuation: key.is_none()
            && !generated
            && (line.starts_with(' ') || line.starts_with('\t')),
    }
}

impl TrailerScan {
    /// Count the pending continuation lines as non-trailer lines.
    fn flush_continuation(&mut self) {
        self.non_trailer_lines += self.continuation_lines;
        self.continuation_lines = 0;
    }

    /// Fold one classified line into the running totals.
    fn absorb(&mut self, line: TrailerLine<'_>, trailer: &str) {
        if line.generated {
            self.trailer_lines += 1;
            self.continuation_lines = 0;
            self.recognized_prefix = true;
        } else if line.key.is_some() {
            self.trailer_lines += 1;
            self.continuation_lines = 0;
        } else if line.continuation {
            self.continuation_lines += 1;
        } else {
            self.non_trailer_lines += 1;
            self.flush_continuation();
        }

        if line
            .key
            .is_some_and(|key| key.eq_ignore_ascii_case(trailer))
        {
            self.matching_trailer = true;
        }
    }
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
///
/// The repository's shallow state and each revision's trailer verdict are
/// repository/commit facts, not file facts, so both are resolved once per
/// selection: many files commonly share one governing commit, and the shallow
/// probe must not run per file.
pub fn select(repo: &Path, files: &[String], cfg: &Config, all: bool) -> Vec<String> {
    select_in_scope(repo, files, cfg, all).files
}

/// What commit-attribution scope did with a candidate list.
///
/// The counts exist for observability only: `files` is exactly what
/// [`select`] returns. A full run resolves no attribution, so its three
/// counters are zero and every candidate is selected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeSelection {
    /// Candidates kept for analysis, in input order.
    pub files: Vec<String>,
    /// Kept because the governing revision carries the configured trailer.
    pub attributed: usize,
    /// Kept because git has no governing revision for the path.
    pub without_commit: usize,
    /// Dropped because the governing revision lacks the trailer.
    pub unattributed: usize,
}

pub fn select_in_scope(repo: &Path, files: &[String], cfg: &Config, all: bool) -> ScopeSelection {
    if all {
        return ScopeSelection {
            files: files.to_vec(),
            ..ScopeSelection::default()
        };
    }

    let trailer = cfg.attribution.trailer.as_str();
    let shallow = is_shallow_repository(repo);
    let mut verdicts: BTreeMap<String, bool> = BTreeMap::new();
    let mut selection = ScopeSelection::default();
    for file in files {
        match governing_revision(repo, file) {
            None => {
                selection.files.push(file.clone());
                selection.without_commit += 1;
            }
            Some(revision) => {
                let attributed = *verdicts.entry(revision.clone()).or_insert_with(|| {
                    governing_commit_has_trailer(repo, &revision, trailer, shallow)
                });
                if attributed {
                    selection.files.push(file.clone());
                    selection.attributed += 1;
                } else {
                    selection.unattributed += 1;
                }
            }
        }
    }
    selection
}

/// Whether the full message of `revision` carries `trailer`; any failure to
/// read the commit is reported as "not attributed".
///
/// `shallow` is the repository-level shallow state from
/// [`is_shallow_repository`]: a shallow-clone boundary commit has no parents
/// and therefore no history that git can consult, so its message is treated as
/// having no trustworthy trailer. Default mode stays conservative and treats
/// it as unattributed; `select(..., all = true)` still includes the file.
fn governing_commit_has_trailer(repo: &Path, revision: &str, trailer: &str, shallow: bool) -> bool {
    if is_shallow_boundary(repo, revision, shallow) {
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

/// The repository's shallow state, asked once per selection.
///
/// Repositories git cannot confirm as shallow (including versions without
/// `--is-shallow-repository`) report `false`, which degrades to the ordinary
/// message check.
fn is_shallow_repository(repo: &Path) -> bool {
    git_command(repo)
        .args(["rev-parse", "--is-shallow-repository"])
        .output()
        .is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true"
        })
}

/// True when `revision` is the history boundary of a shallow clone.
///
/// In a shallow clone the boundary commit is presented as parentless, so
/// `rev-list --parents -n 1` printing only the commit itself marks the cut
/// point. `shallow` is the repository-level answer; a non-shallow repository
/// needs no `rev-list` probe at all.
fn is_shallow_boundary(repo: &Path, revision: &str, shallow: bool) -> bool {
    if !shallow {
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
