//! Purpose: own the machine output of `sad check` — the report payload carried
//! as `data` and the gate failures the CLI renders as error envelopes.
//! Responsibilities: define [`CheckReport`], [`FileCounts`], and
//! [`CheckStats`] (the serialized contract: `findings`, `suppression_debt`,
//! and `stats`) with [`CheckStats::of`] counting one run, define
//! [`GateFailure`] with its stable error codes and [`growth_message`] wording,
//! and build the genesis `Check` envelope, setting `ok` so the envelope
//! verdict and the process exit status agree.
//! Rationale: genesis 0.13.2 hardcodes `ok = true` for every non-error kind,
//! but a check envelope with new findings must fail — so the verdict is
//! derived here from the severity counts (`strict` refuses every finding, the
//! default refuses only error-severity ones) and written explicitly, keeping
//! the envelope the only place that decides pass/fail. Gate failures live
//! beside the payload they replace so the success and error contracts stay
//! together.

use std::fmt;
use std::path::Path;

use genesis::envelope::{Envelope, EnvelopeKind, Warning};
use serde::Serialize;

use crate::baseline::BaselineError;
use crate::exemptions;
use crate::model::{AnalyzedFile, AttributionClass, Finding, Severity, SuppressionEntry};

/// Machine-readable body of the `sad check` envelope.
///
/// Field names are the contract: `findings`, `suppression_debt`, and `stats`
/// with one count per channel plus the run totals.
#[derive(Debug, Clone, Serialize)]
pub struct CheckReport {
    pub findings: Vec<Finding>,
    pub suppression_debt: Vec<SuppressionEntry>,
    pub stats: CheckStats,
    /// Envelope-level warnings the pipeline collected (unsupported
    /// extensions). Lifted into `Envelope::warnings` by
    /// [`check_envelope`]; never serialized inside the data payload.
    #[serde(skip)]
    pub warnings: Vec<Warning>,
}

/// File-level counts from discovery and selection, before extraction.
///
/// Kept apart from [`CheckStats`] because they describe the pipeline, not the
/// analysed tree; the run fills them in once selection and extraction finish.
#[derive(Debug, Clone, Copy, Default)]
pub struct FileCounts {
    /// Source files discovery found, before exclusion and scope.
    pub discovered: usize,
    /// Files left after exclusion and commit scope that extraction attempted.
    pub selected: usize,
    /// Selected files dropped because they were unreadable or not UTF-8.
    pub skipped: usize,
    /// Discovered files an `exclude` glob dropped.
    pub excluded: usize,
    /// Selected because the governing revision carries the trailer.
    pub attributed: usize,
    /// Selected because git has no governing revision for the path.
    pub without_commit: usize,
    /// Candidates dropped because the governing revision lacks the trailer.
    pub unattributed: usize,
    /// Discovered files whose extension the gate cannot parse.
    pub unsupported: usize,
}

/// Counts describing what one gate run looked at.
///
/// The `comments_*` channels count nodes on the same population as
/// `files_analyzed` — successfully analysed, non-exempt files — so per-file
/// rates derived from the envelope share one denominator. `exempt_nodes`
/// alone spans every analysed file, exempt ones included, because exemption
/// nodes are exactly the evidence a whole-file exemption is made of.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CheckStats {
    /// Source files discovery found, before exclusion and scope.
    pub files_discovered: usize,
    /// Discovered files an `exclude` glob dropped.
    pub files_excluded: usize,
    /// Files left in scope after exclusion; extraction attempted each one.
    pub files_selected: usize,
    /// Selected files skipped as unreadable or not UTF-8.
    pub files_skipped: usize,
    /// Candidates selected because their governing revision carries the
    /// configured attribution trailer.
    pub files_attributed: usize,
    /// Candidates selected because git has no governing revision for them.
    pub files_without_commit: usize,
    /// Candidates dropped because their governing revision lacks the trailer.
    pub files_unattributed: usize,
    /// Discovered files skipped because their extension is unsupported.
    pub files_unsupported: usize,
    /// Non-exempt files analysed successfully — the channel denominator.
    pub files_analyzed: usize,
    /// Analysed files the extractor marked exempt.
    pub files_exempt: usize,
    /// Exemption-class comment nodes across every analysed file.
    pub exempt_nodes: usize,
    pub comments_header: usize,
    pub comments_doc: usize,
    pub comments_marker: usize,
    pub comments_rationale: usize,
    pub comments_suppression: usize,
    pub comments_prose: usize,
    pub suppression_debt: usize,
    /// Findings that reached the report, i.e. new debt for this run.
    pub findings: usize,
    /// How many of those findings carry [`Severity::Error`]; filled by
    /// [`check_envelope`] from the released findings, so the default and
    /// `--strict` verdicts share one count.
    pub findings_blocking: usize,
    /// Findings grandfathered by `sad-baseline.json` and not reported.
    pub findings_suppressed: usize,
}

impl CheckStats {
    /// Counts for one run: file and exemption totals, per-channel comment
    /// classes, suppression debt, and the findings that reached the report.
    pub fn of(
        analyzed: &[AnalyzedFile],
        debt: &[SuppressionEntry],
        findings: &[Finding],
        files: FileCounts,
    ) -> Self {
        let summary = exemptions::summary(analyzed);
        Self {
            files_discovered: files.discovered,
            files_excluded: files.excluded,
            files_selected: files.selected,
            files_skipped: files.skipped,
            files_attributed: files.attributed,
            files_without_commit: files.without_commit,
            files_unattributed: files.unattributed,
            files_unsupported: files.unsupported,
            files_analyzed: analyzed.len() - summary.files_exempt,
            files_exempt: summary.files_exempt,
            exempt_nodes: summary.nodes_exempt,
            comments_header: count_class(analyzed, AttributionClass::Header),
            comments_doc: count_class(analyzed, AttributionClass::Doc),
            comments_marker: count_class(analyzed, AttributionClass::Marker),
            comments_rationale: count_class(analyzed, AttributionClass::Rationale),
            comments_suppression: count_class(analyzed, AttributionClass::Suppression),
            comments_prose: count_class(analyzed, AttributionClass::Prose),
            suppression_debt: debt.len(),
            findings: findings.len(),
            findings_blocking: 0,
            findings_suppressed: 0,
        }
    }
}

/// Count how many comments on non-exempt analysed files were attributed to
/// `class` — the same population `files_analyzed` counts.
fn count_class(files: &[AnalyzedFile], class: AttributionClass) -> usize {
    files
        .iter()
        .filter(|file| !file.exempt)
        .flat_map(|file| file.nodes.iter())
        .filter(|node| node.class == class)
        .count()
}

/// Why a `sad check` run failed before it could report findings.
///
/// These are gate failures, not prose findings: the CLI renders them as an
/// error envelope with non-empty remediation so a broken baseline is never
/// confused with documentation debt.
#[derive(Debug)]
pub enum GateFailure {
    /// `sad-baseline.json` could not be read, parsed, or written.
    Baseline(BaselineError),
    /// Adoption would add ids that are not in the existing baseline,
    /// violating the shrink-only invariant.
    Growth { message: String },
    /// `.beads/issues.jsonl` exists, is non-empty, and yielded no record — the
    /// resolver source is broken, not the comments it would judge.
    Ledger { message: String },
}

impl GateFailure {
    /// Stable error code for the envelope.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Baseline(_) => "baseline_error",
            Self::Growth { .. } => "baseline_growth",
            Self::Ledger { .. } => "ledger_error",
        }
    }
}

impl fmt::Display for GateFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Baseline(error) => write!(f, "{error}"),
            Self::Growth { message } | Self::Ledger { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for GateFailure {}

/// Human-readable adoption-growth failure naming the baseline path and ids.
///
/// Only adoption can violate shrink-only: a plain run never compares the
/// baseline against the tree, so fixed prose cannot fail as "stale".
pub fn growth_message(path: &Path, ids: &[String]) -> String {
    let listing = ids.join(", ");
    format!(
        "refusing to grow baseline `{}`: {} finding id(s) are not in the existing baseline \
         ({listing}); the baseline is shrink-only, so fix those findings instead of \
         adopting them",
        path.display(),
        ids.len()
    )
}

/// Build the Check envelope for a completed run.
///
/// A clean run and a warning-only run pass by default; any error-severity
/// finding fails, and `strict` fails on any finding at all. The payload stays
/// a successful command result rather than an error envelope.
pub fn check_envelope(mut report: CheckReport, strict: bool) -> Envelope<CheckReport> {
    report.stats.findings_blocking = report
        .findings
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .count();
    let ok = if strict {
        report.findings.is_empty()
    } else {
        report.stats.findings_blocking == 0
    };
    let warnings = std::mem::take(&mut report.warnings);
    let mut envelope = Envelope::success(
        env!("CARGO_PKG_VERSION"),
        EnvelopeKind::Check,
        report,
        warnings,
        vec![],
    );
    envelope.ok = ok;
    envelope
}
