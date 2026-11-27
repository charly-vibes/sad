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

use genesis::envelope::{Envelope, EnvelopeKind, RemediationEntry};

use crate::envelope::spec_error_envelope;

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

/// What one `sad hooks uninstall` run did to each lefthook stage.
///
#[derive(Debug, Serialize)]
pub struct HooksUninstall {
    /// Outcome for the `pre-commit` stage.
    pub pre_commit: UninstallOutcome,
    /// Outcome for the `pre-push` stage.
    pub pre_push: UninstallOutcome,
}

/// Per-stage result of one `sad hooks uninstall` run.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UninstallOutcome {
    /// The stage carried the sad managed block and it was removed.
    Removed,
    /// The stage carried no sad managed block, so nothing changed.
    NotWired,
}

/// Build the `sad hooks install` envelope: a successful command result
/// carrying the per-stage outcomes, never findings.
pub(crate) fn hooks_envelope(install: HooksInstall) -> Envelope<HooksInstall> {
    Envelope::success(
        env!("CARGO_PKG_VERSION"),
        EnvelopeKind::Ok,
        install,
        vec![],
        vec![],
    )
}

/// Build the `sad hooks uninstall` envelope: a successful command result
/// carrying the per-stage outcomes, never findings.
pub(crate) fn hooks_uninstall_envelope(uninstall: HooksUninstall) -> Envelope<HooksUninstall> {
    Envelope::success(
        env!("CARGO_PKG_VERSION"),
        EnvelopeKind::Ok,
        uninstall,
        vec![],
        vec![],
    )
}

/// Spec-traced failure envelope for a refused hooks command; the remediation
/// names the concrete blocker (no repository, no lefthook config, or an
/// unusable config) per Invariant 3.2.5. `kind` and `command` carry the
/// calling subcommand so each refusal traces to its own surface.
pub(crate) fn hooks_failure_envelope(
    error: &lefthook::HooksError,
    kind: &str,
    command: &str,
) -> serde_json::Value {
    let (message, remediation) = failure_parts(error, command);
    spec_error_envelope(kind, &message, "cli", vec![remediation])
}

/// The message and single remediation naming the concrete blocker (no
/// repository, no lefthook config, or an unusable config) per Invariant
/// 3.2.5; `command` carries the calling subcommand into the remediation so
/// each refusal traces to its own surface.
fn failure_parts(error: &lefthook::HooksError, command: &str) -> (String, RemediationEntry) {
    use lefthook::HooksError;
    match error {
        HooksError::NotInRepo { .. } => (
            error.to_string(),
            RemediationEntry {
                command: "git init".into(),
                description: format!(
                    "Hook wiring targets a git repository; run {command} inside one."
                ),
            },
        ),
        HooksError::Framework { message } => (
            message.clone(),
            RemediationEntry {
                command: "lefthook init".into(),
                description:
                    "Create a lefthook.yml; sad wires managed blocks and never creates one.".into(),
            },
        ),
        HooksError::Io { .. } => (
            error.to_string(),
            RemediationEntry {
                command: command.into(),
                description: "Fix the filesystem error and re-run the command.".into(),
            },
        ),
    }
}
