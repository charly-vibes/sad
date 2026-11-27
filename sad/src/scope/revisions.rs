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
    let Some(stdout) = log_stdout(repo, rel_paths) else {
        return revisions;
    };
    let indices = requested_indices(rel_paths);
    assign_first_occurrences(&stdout, &indices, &mut revisions, rel_paths);
    revisions
}

/// The successful stdout of one full-history `git log -z` pass, or `None`
/// when there is nothing to resolve or git failed.
///
/// No requested paths means no history to resolve; a git failure or a
/// non-zero exit degrades to `None`, which the caller renders as "no
/// governing commit" for every path.
fn log_stdout(repo: &Path, rel_paths: &[String]) -> Option<Vec<u8>> {
    if rel_paths.is_empty() {
        return None;
    }
    let output = git_command(repo)
        .args([
            "log",
            "--no-show-signature",
            "--format=%H",
            "--name-only",
            "--relative",
            "-z",
        ])
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
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

/// Record each requested path's first mentioning commit: walk the `-z` stream
/// newest-first, and later occurrences never overwrite earlier ones.
fn assign_first_occurrences(
    stdout: &[u8],
    indices: &BTreeMap<&[u8], usize>,
    revisions: &mut BTreeMap<String, Option<String>>,
    rel_paths: &[String],
) {
    for (revision, token) in tokens_under_governing_commit(stdout) {
        let path = record_path(token);
        let Some(&index) = indices.get(path) else {
            continue;
        };
        let Some(slot @ None) = revisions.get_mut(&rel_paths[index]) else {
            continue;
        };
        *slot = Some(revision.to_string());
    }
}

/// Each non-hash `-z` token paired with the newest commit hash seen before it.
///
/// A hash token opens a record and everything after it belongs to that commit
/// until the next hash. A token that is itself exactly 40 hex digits is
/// indistinguishable from a hash and is treated as one — the documented
/// ambiguity that degrades such a path to "no governing commit".
fn tokens_under_governing_commit(stdout: &[u8]) -> impl Iterator<Item = (&str, &[u8])> {
    stdout
        .split(|byte| *byte == 0)
        .scan(None::<&str>, |governing, token| {
            if is_hash(token) {
                *governing = std::str::from_utf8(token).ok();
                Some(None)
            } else {
                Some((*governing).map(|revision| (revision, token)))
            }
        })
        .flatten()
}

/// Whether a `-z` token is a commit hash: exactly 40 ASCII hex digits.
fn is_hash(token: &[u8]) -> bool {
    token.len() == 40 && token.iter().all(u8::is_ascii_hexdigit)
}

/// A record's path token: the log format's newline before a file name is
/// framing, not content.
fn record_path(token: &[u8]) -> &[u8] {
    token.strip_prefix(b"\n").unwrap_or(token)
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
