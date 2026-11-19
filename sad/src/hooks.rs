//! Purpose: `sad hooks` subcommands — adopt-time wiring of the sad gate into
//! repository hook managers.
//! Responsibilities: expose the lefthook installer behind the `sad hooks
//! install` CLI surface, carry the per-stage outcomes the CLI serializes
//! into a suite envelope, and keep hook adoption explicit, idempotent, and
//! observable.
//! Rationale: the wiring mechanics live in genesis
//! (`git_hooks::lefthook::ensure_wired`), while the marker, the gate
//! command, and the error contracts are sad-owned — the boundary rule puts
//! no tool gate commands in genesis, so the installer is the single place
//! where "what the hook runs" is decided.

pub mod lefthook;

use serde::Serialize;

/// What one `sad hooks install` run did to each lefthook stage.
///
#[derive(Debug, Serialize)]
pub struct HooksInstall {
    /// Outcome for the `pre-commit` stage.
    pub pre_commit: StageOutcome,
    /// Outcome for the `pre-push` stage.
    pub pre_push: StageOutcome,
}

/// Per-stage result of one `sad hooks install` run.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StageOutcome {
    /// The managed block was inserted into the stage.
    Injected,
    /// The managed block was already present, so nothing changed.
    AlreadyWired,
    /// The stage already runs the gate outside the managed block, and
    /// wiring it again would run the gate twice.
    SkippedUserGate,
}
