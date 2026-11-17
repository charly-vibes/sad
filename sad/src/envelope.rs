//! Purpose: turn a finished check report into the genesis `Check` suite
//! envelope — the single machine output of `sad check`.
//! Responsibilities: pick [`EnvelopeKind::Check`], carry the report as `data`,
//! and set `ok` so the envelope verdict and the process exit status agree.
//! Rationale: genesis 0.13.2 hardcodes `ok = true` for every non-error kind,
//! but a check envelope with findings must be a failure — so the verdict is
//! derived here from `findings.is_empty()` and written explicitly, keeping the
//! envelope the only place that decides pass/fail.

use genesis::envelope::{Envelope, EnvelopeKind};

use crate::check::CheckReport;

/// Build the Check envelope for a completed run.
///
/// No findings is a pass; any finding is a fail, even though the payload is a
/// successful command result rather than an error envelope.
pub fn check_envelope(report: CheckReport) -> Envelope<CheckReport> {
    let ok = report.findings.is_empty();
    let mut envelope = Envelope::success(
        env!("CARGO_PKG_VERSION"),
        EnvelopeKind::Check,
        report,
        vec![],
        vec![],
    );
    envelope.ok = ok;
    envelope
}
