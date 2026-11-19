//! Purpose: build the marker resolver context from the repository's ledgers.
//! Responsibilities: honour the `[resolvers]` toggles, parse the beads JSONL
//! ledger (rejecting a non-empty ledger that yields no id by naming it), and
//! collect openspec ids from `openspec/specs/` plus the capability directories
//! declared by pending change deltas.
//! Rationale: a marker is judged only against sources the run may consult, so
//! a disabled resolver must be indistinguishable from an absent one; and a
//! broken export must fail the gate naming the ledger instead of turning every
//! marker into a finding whose remediation blames the comment.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::checks::RepoContext;
use crate::config::Config;
use crate::envelope::GateFailure;
use crate::model::{AnalyzedFile, AttributionClass};

use super::BEADS_LEDGER;

/// OpenSpec specs directory, relative to the repository root.
const OPENSPEC_SPECS: &str = "openspec/specs";

/// OpenSpec change deltas, relative to the repository root — each change may
/// declare spec/capability ids (for example `comment-gate`) before they are
/// synced into `openspec/specs/`.
const OPENSPEC_CHANGES: &str = "openspec/changes";

/// Build the marker resolver context only when a check will consult it.
///
/// Marker resolution is the only consumer, so the ledgers are read only while
/// the `unresolvable_marker` check is enabled *and* at least one analysed file
/// carries a marker node; every other run gets [`RepoContext::empty`] and no
/// ledger I/O.
pub(super) fn resolver_context(
    repo: &Path,
    cfg: &Config,
    analyzed: &[AnalyzedFile],
) -> Result<RepoContext, GateFailure> {
    let has_markers = analyzed.iter().any(|file| {
        file.nodes
            .iter()
            .any(|node| node.class == AttributionClass::Marker)
    });
    if !cfg.checks.unresolvable_marker || !has_markers {
        return Ok(RepoContext::empty());
    }
    repo_context(repo, cfg)
}

/// Build the marker resolver context from the repository's ledgers.
///
/// Each resolver is consulted only while its `[resolvers]` toggle is on: a
/// disabled source is treated as absent, so it can neither resolve an id nor
/// (for beads) satisfy the availability check that guards marker judgement.
/// The beads ledger is available when `.beads/issues.jsonl` can be read; every
/// JSONL line is parsed independently, so one corrupt record cannot hide the
/// rest. The openspec resolver is available when `openspec/specs/` exists; its
/// ids are the spec directory names plus the capability directories declared
/// by pending change deltas (`openspec/changes/*/specs/*`).
fn repo_context(repo: &Path, cfg: &Config) -> Result<RepoContext, GateFailure> {
    let (beads_ids, beads_available) = beads_context(repo, cfg.resolvers.beads)?;
    let (openspec_ids, openspec_available) = openspec_context(repo, cfg.resolvers.openspec);
    Ok(RepoContext {
        beads_ids,
        openspec_ids,
        beads_available,
        openspec_available,
    })
}

/// Read the beads ledger when its resolver is enabled.
///
/// A missing ledger is the documented no-op, not an error. A ledger that exists
/// but yields no id is a gate failure naming the file: treating it as available
/// with an empty id set would turn every marker into a finding whose
/// remediation blames the comment instead of the broken export. The same
/// refusal covers an existing ledger that cannot be read at all (unreadable
/// or undecodable): availability must never silently depend on read errors.
fn beads_context(repo: &Path, enabled: bool) -> Result<(BTreeSet<String>, bool), GateFailure> {
    if !enabled {
        return Ok((BTreeSet::new(), false));
    }
    let ledger_path = repo.join(BEADS_LEDGER);
    if !ledger_path.exists() {
        return Ok((BTreeSet::new(), false));
    }
    let Ok(text) = fs::read_to_string(&ledger_path) else {
        return Err(GateFailure::Ledger {
            message: format!(
                "beads ledger `{BEADS_LEDGER}` exists but cannot be read or decoded; the \
                 resolver cannot judge any marker",
            ),
        });
    };
    let ledger = parse_beads_ids(&text);
    if ledger.records == 0 && ledger.non_blank_lines > 0 {
        return Err(GateFailure::Ledger {
            message: format!(
                "beads ledger `{BEADS_LEDGER}` holds {} non-empty line(s) but no record with an \
                 `id`; the resolver cannot judge any marker",
                ledger.non_blank_lines
            ),
        });
    }
    Ok((ledger.ids, true))
}

/// Collect openspec spec ids when its resolver is enabled; a disabled resolver
/// is reported as absent however many spec directories exist.
fn openspec_context(repo: &Path, enabled: bool) -> (BTreeSet<String>, bool) {
    let specs_dir = repo.join(OPENSPEC_SPECS);
    let available = enabled && specs_dir.is_dir();
    let mut ids = BTreeSet::new();
    if available {
        collect_directory_names(&specs_dir, &mut ids);
        collect_change_specs(&repo.join(OPENSPEC_CHANGES), &mut ids);
    }
    (ids, available)
}

/// Add the capability directories declared by pending openspec change deltas,
/// visiting changes in sorted order so the resolver set is reproducible.
fn collect_change_specs(changes_dir: &Path, ids: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(changes_dir) else {
        return;
    };
    let mut changes: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    changes.sort();
    for change in changes {
        collect_directory_names(&change.join("specs"), ids);
    }
}

/// Outcome of parsing the beads ledger.
///
/// The id set alone cannot distinguish an empty ledger from a corrupt one, so
/// the parse also records how much content it saw and how many records carried
/// an id — the two numbers [`beads_context`] needs to refuse a broken export.
#[derive(Debug, Default)]
struct BeadsLedger {
    /// Ids declared by records that carried a string `id`.
    ids: BTreeSet<String>,
    /// Lines that hold content; blank lines are not evidence of a ledger.
    non_blank_lines: usize,
    /// Lines that parsed as a record with a string `id`.
    records: usize,
}

/// Parse beads JSONL into the declared issue ids, ignoring malformed lines and
/// records without a string `id` while counting both lines and id records.
fn parse_beads_ids(text: &str) -> BeadsLedger {
    let mut ledger = BeadsLedger::default();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        ledger.non_blank_lines += 1;
        let Ok(record) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(id) = record.get("id").and_then(serde_json::Value::as_str) else {
            continue;
        };
        ledger.ids.insert(id.to_string());
        ledger.records += 1;
    }
    ledger
}

/// Add every directory name directly under `dir` to `ids`.
fn collect_directory_names(dir: &Path, ids: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            ids.insert(entry.file_name().to_string_lossy().into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_beads_lines_do_not_hide_valid_ids() {
        let ledger = parse_beads_ids("not json\n{\"id\":\"sad-a\"}\n{\"title\":\"no id\"}\n");
        assert_eq!(ledger.ids, BTreeSet::from(["sad-a".to_string()]));
        assert_eq!(ledger.non_blank_lines, 3);
        assert_eq!(ledger.records, 1);
    }

    #[test]
    fn a_non_empty_ledger_with_no_id_record_is_detectable() {
        let ledger = parse_beads_ids("{\"title\":\"truncated\"\n{\"title\":\"no id\"}\n\n");
        assert!(ledger.ids.is_empty());
        assert_eq!(ledger.non_blank_lines, 2);
        assert_eq!(ledger.records, 0);
    }
}

    #[test]
    fn existing_but_unreadable_ledger_fails_and_missing_is_a_no_op() {
        let dir = std::env::temp_dir().join(format!("sad-resolvers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".beads")).unwrap();
        let ledger = dir.join(BEADS_LEDGER);
        std::fs::write(&ledger, b"\xff\xfe not utf-8").unwrap();
        // An existing ledger that cannot be decoded must fail the gate:
        // silently disabling marker judgement would blame comments instead
        // of the broken ledger.
        let outcome = beads_context(&dir, true);
        assert!(outcome.is_err(), "expected GateFailure, got {outcome:?}");
        std::fs::remove_file(&ledger).unwrap();
        let (ids, available) = beads_context(&dir, true).unwrap();
        assert!(!ids.is_empty() || !available, "missing ledger is the documented no-op");
        std::fs::remove_dir_all(&dir).unwrap();
    }
