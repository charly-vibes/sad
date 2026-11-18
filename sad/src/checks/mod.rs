//! Purpose: the check-module contract shared by every gate check.
//! Responsibilities: define [`RepoContext`] (the repo ledgers a marker can
//! resolve against, plus whether each ledger exists) and re-export the
//! per-check modules.
//! Rationale: checks must be pure functions of an analyzed file plus config
//! and repo ledgers; keeping the context type here lets each check module stay
//! independent and table-testable without touching the filesystem.

use std::collections::BTreeSet;

pub mod commented_out_code;
pub mod doc_cap;
pub mod header;
pub mod marker;
pub mod outside_channel;
pub mod suppression;

/// Repository ledgers a marker id may resolve against.
///
/// The `*_available` flags are the no-op contract: a resolver whose source is
/// absent (`None` file, missing directory) or disabled by `[resolvers]` must
/// not turn every marker into a finding, so checks consult the flag before
/// judging an id unresolvable.
#[derive(Debug, Default, Clone)]
pub struct RepoContext {
    /// Ids found in `.beads/issues.jsonl`.
    pub beads_ids: BTreeSet<String>,
    /// Ids declared under `openspec/specs/`.
    pub openspec_ids: BTreeSet<String>,
    /// Whether a beads ledger exists for this run.
    pub beads_available: bool,
    /// Whether the openspec specs directory exists for this run.
    pub openspec_available: bool,
}

impl RepoContext {
    /// Context with both resolvers absent — the no-op baseline.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Whether any resolver source exists; false means marker resolution is
    /// skipped entirely for this run.
    pub fn any_resolver_available(&self) -> bool {
        self.beads_available || self.openspec_available
    }

    /// Whether `id` resolves against an available ledger.
    pub fn resolves(&self, id: &str) -> bool {
        (self.beads_available && self.beads_ids.contains(id))
            || (self.openspec_available && self.openspec_ids.contains(id))
    }
}
