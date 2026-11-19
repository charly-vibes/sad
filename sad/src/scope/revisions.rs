//! Purpose: resolve governing revisions — the last commit that touched each
//! candidate path — in one git process.
//! Responsibilities: stream the full history newest-first through a single
//! `-z`-framed `git log`, assign each requested path its first-mentioning
//! commit, and degrade every failure (unreadable history, absent path,
//! undecodable hash) to "no governing commit".
//! Rationale: default scope is the adoption guard, and one process per
//! candidate file made every run spawn one git process per file; the batch
//! pass keeps the newest-first first-occurrence rule, literal path
//! semantics via `--relative`, and the documented degrade instead of
//! panicking. Paths are relative to `repo` (which may be a subdirectory of
//! the actual repository; `--relative` keeps the stream in the same
//! convention). `-z` framing keeps every byte of a path exact except that
//! a file named exactly 40 hex characters is indistinguishable from a
//! commit hash and degrades to "no governing commit" — the path stays in
//! scope either way.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// The full hash of the last commit that touched each of `rel_paths`, or
/// `None` for paths git has never recorded.
///
/// History output is bytes, not UTF-8 — only the hashes need decoding, so
/// requested paths are matched as raw bytes against the stream. An entry
/// opens with its 40-hex commit hash; the log format's newline reaches us
/// as the first byte of the next token, which is then a file name.
pub fn governing_revisions(repo: &Path, rel_paths: &[String]) -> BTreeMap<String, Option<String>> {
    let mut revisions: BTreeMap<String, Option<String>> = rel_paths
        .iter()
        .map(|rel_path| (rel_path.clone(), None))
        .collect();
    if rel_paths.is_empty() {
        return revisions;
    }
    let indices = requested_indices(rel_paths);
    let output = match git_command(repo)
        .args([
            "log",
            "--no-show-signature",
            "--format=%H",
            "--name-only",
            "--relative",
            "-z",
        ])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return revisions,
    };
    assign_first_occurrences(&output.stdout, &indices, &mut revisions, rel_paths);
    revisions
}

/// Requested paths as byte slices with their positions: history output is
/// bytes, not UTF-8, and only the hashes need decoding.
fn requested_indices(rel_paths: &[String]) -> BTreeMap<&[u8], usize> {
    rel_paths
        .iter()
        .enumerate()
        .map(|(index, rel_path)| (rel_path.as_bytes(), index))
        .collect()
}

/// Walk the `-z` stream newest-first and record each requested path's first
/// mentioning commit. Later occurrences never overwrite earlier ones.
fn assign_first_occurrences(
    stdout: &[u8],
    indices: &BTreeMap<&[u8], usize>,
    revisions: &mut BTreeMap<String, Option<String>>,
    rel_paths: &[String],
) {
    let mut governing: Option<&str> = None;
    for token in stdout.split(|byte| *byte == 0) {
        if token.len() == 40 && token.iter().all(u8::is_ascii_hexdigit) {
            governing = std::str::from_utf8(token).ok();
            continue;
        }
        let Some(revision) = governing else { continue };
        let path = token.strip_prefix(b"\n").unwrap_or(token);
        if let Some(&index) = indices.get(path)
            && let Some(slot @ None) = revisions.get_mut(&rel_paths[index])
        {
            *slot = Some(revision.to_string());
        }
    }
}

/// A git command rooted at `repo` with inherited repository overrides removed
/// so the requested path always wins, and pathspec globbing disabled so a
/// repo-relative path is always interpreted as the literal file name.
pub(crate) fn git_command(repo: &Path) -> Command {
    let mut command = Command::new("git");
    command.args(["--literal-pathspecs", "-C"]).arg(repo);
    command
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    command
}
