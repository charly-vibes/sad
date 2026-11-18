//! Purpose: run the whole documentation gate over a repository tree — file
//! discovery, exclusion, commit scoping, extraction, every enabled check, the
//! baseline split, and the stats and suppression-debt the envelope reports.
//! Responsibilities: collect supported source files deterministically, apply
//! `exclude` globs and commit-attribution scope, report what discovery,
//! exclusion, and extraction dropped through the stats coverage counters and
//! one `unreadable_source` warning per selected file that could not be read,
//! build the marker resolver context lazily from the enabled resolver sources
//! (beads ledger, openspec specs), run each check on non-exempt files, order
//! findings by path, line, and check id, then load `sad-baseline.json` to
//! grandfather adopted ids, adopt on request, and enforce the shrink-only
//! invariant at adoption time.
//! Rationale: the envelope is only trustworthy if its inputs are reproducible,
//! so discovery is a pure tree walk with no clock, network, or parallelism and
//! every collection is sorted before it reaches the report. Unreadable files
//! are skipped deterministically and named by path and reason, so a passing
//! run never hides a source it could not judge; an I/O failure is not a
//! documentation finding, and the gate reports on what it can parse without
//! letting the machine's I/O state change the findings.
//!
//! Stages: [`select_files`] (discover, exclude, scope) → [`analyze_files`]
//! (extraction) → [`collect_debt`] and [`collect_findings`] (checks) →
//! [`CheckStats::of`] (stats) → [`apply_baseline`], which loads
//! `sad-baseline.json` and delegates to [`adopt_baseline`] or
//! [`suppress_baselined`].

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::baseline::{self, Baseline};
use genesis::envelope::Warning;

use crate::checks::{
    RepoContext, commented_out_code, doc_cap, header, marker, outside_channel, smell, suppression,
};
use crate::config::Config;
use crate::envelope::{CheckReport, CheckStats, FileCounts, GateFailure, growth_message};
use crate::exemptions;
use crate::extract;
use crate::model::{AnalyzedFile, Finding, SuppressionEntry};
use crate::scope;

use self::discovery::discover_files;
use self::resolvers::resolver_context;
use self::warnings::{skipped_directory_warnings, unsupported_warnings};

mod discovery;
mod resolvers;
mod warnings;

/// Beads ledger path, relative to the repository root.
pub const BEADS_LEDGER: &str = ".beads/issues.jsonl";

/// Analyse `repo`, apply the baseline gate, and build the check report.
///
/// `all` bypasses commit-attribution scope; configuration toggles and caps are
/// honoured by the individual checks. `adopt` implies `all`, so adoption
/// always sees the whole tree. With `adopt`, the current findings are
/// persisted as the baseline (bounded by the shrink-only invariant) and the
/// run reports a clean pass. Otherwise ids present in `sad-baseline.json` are
/// suppressed, while genuinely new findings are reported and fail the run.
pub fn run(repo: &Path, cfg: &Config, all: bool, adopt: bool) -> Result<CheckReport, GateFailure> {
    let all = all || adopt;
    let selection = select_files(repo, cfg, all);
    let (analyzed, skipped) = analyze_files(repo, &selection.selected, cfg);
    let debt = collect_debt(&analyzed);
    let context = resolver_context(repo, cfg, &analyzed)?;
    let findings = collect_findings(&analyzed, cfg, &context);
    let files = FileCounts {
        discovered: selection.discovered,
        selected: selection.selected.len(),
        skipped: skipped.len(),
        excluded: selection.excluded,
        attributed: selection.attributed,
        without_commit: selection.without_commit,
        unattributed: selection.unattributed,
        unsupported: selection.unsupported.len(),
    };
    let stats = CheckStats::of(&analyzed, &debt, &findings, files);
    let mut warnings = unsupported_warnings(&selection.unsupported);
    warnings.extend(skipped_warnings(&skipped));
    warnings.extend(skipped_directory_warnings(&selection.skipped_directories));
    let report = CheckReport {
        findings,
        suppression_debt: debt,
        stats,
        warnings,
    };
    apply_baseline(repo, report, adopt)
}

/// What discovery and selection kept and dropped for one run.
struct Selection {
    /// Source files discovery found, before exclusion and scope.
    discovered: usize,
    /// Discovered files an `exclude` glob dropped.
    excluded: usize,
    /// Files that survived exclusion and commit scope, in discovery order.
    selected: Vec<String>,
    attributed: usize,
    without_commit: usize,
    unattributed: usize,
    unsupported: Vec<String>,
    skipped_directories: Vec<String>,
}

/// Files this run analyses: the discovered tree minus `exclude` globs, scoped
/// to the commit-attribution range unless `all` bypasses it.
fn select_files(repo: &Path, cfg: &Config, all: bool) -> Selection {
    let discovery = discover_files(repo);
    let discovered = discovery.files;
    let excluded_count = discovered.len();
    let candidates: Vec<String> = discovered
        .into_iter()
        .filter(|file| !exemptions::is_excluded(file, cfg))
        .collect();
    let excluded = excluded_count - candidates.len();
    let scoped = scope::select_in_scope(repo, &candidates, cfg, all);
    Selection {
        discovered: excluded_count,
        excluded,
        selected: scoped.files,
        attributed: scoped.attributed,
        without_commit: scoped.without_commit,
        unattributed: scoped.unattributed,
        unsupported: discovery.unsupported,
        skipped_directories: discovery.skipped,
    }
}

/// One selected source file the run could not read as UTF-8.
///
/// The reason is a short stable phrase so the envelope and the text render
/// say the same thing on every machine; an unusual I/O error keeps the
/// operating-system message instead of being flattened to "unreadable".
struct SkippedSource {
    path: String,
    reason: String,
}

/// Extract every selected file and collect the ones that could not be read.
///
/// Unreadable or non-UTF-8 sources are skipped rather than invented as
/// findings. Each skip is returned with its path and reason so the run can
/// report it, and `stats.files_skipped` counts them.
///
/// One [`extract::Analyzer`] (compiled marker matcher plus one parser per
/// language) is reused across files, so the run compiles the marker pattern
/// and grammar setup once instead of per file.
fn analyze_files(
    repo: &Path,
    selected: &[String],
    cfg: &Config,
) -> (Vec<AnalyzedFile>, Vec<SkippedSource>) {
    let mut analyzer = extract::Analyzer::new(cfg);
    let mut analyzed: Vec<AnalyzedFile> = Vec::new();
    let mut skipped: Vec<SkippedSource> = Vec::new();
    for rel_path in selected {
        let source = match fs::read_to_string(repo.join(rel_path)) {
            Ok(source) => source,
            Err(error) => {
                skipped.push(SkippedSource {
                    path: rel_path.clone(),
                    reason: skip_reason(&error),
                });
                continue;
            }
        };
        analyzed.push(analyzer.analyze(Path::new(rel_path), rel_path, &source, cfg));
    }
    (analyzed, skipped)
}

/// Stable reason phrase for a read failure.
///
/// `InvalidData` is what `read_to_string` reports for non-UTF-8 bytes, which
/// covers latin-1, UTF-16, and corrupted sources; the common permission and
/// disappearance cases get their own phrase. Anything else keeps the OS
/// message so an unexpected failure is still diagnosable.
fn skip_reason(error: &std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::InvalidData => "not valid UTF-8".to_string(),
        std::io::ErrorKind::PermissionDenied => "permission denied".to_string(),
        std::io::ErrorKind::NotFound => "file disappeared during the run".to_string(),
        _ => format!("unreadable ({error})"),
    }
}

/// One envelope warning per skipped selected file, naming its path and reason.
///
/// The warning channel matches `unsupported_extension`: JSON consumers get a
/// per-path record, and the text render reads the same warnings.
fn skipped_warnings(skipped: &[SkippedSource]) -> Vec<Warning> {
    skipped
        .iter()
        .map(|source| Warning {
            rule_name: "unreadable_source".to_string(),
            entity_id: Some(source.path.clone()),
            message: source.reason.clone(),
            suggested_remediation: Some(
                "Make the source readable as UTF-8; the gate never guesses an encoding."
                    .to_string(),
            ),
        })
        .collect()
}

/// Suppression-debt entries from every analysed file, whole-file exempt ones
/// included: debt is observable even where findings are not.
fn collect_debt(analyzed: &[AnalyzedFile]) -> Vec<SuppressionEntry> {
    analyzed.iter().flat_map(suppression::entries).collect()
}

/// Run every enabled check over the non-exempt files and order the findings by
/// path, line, and check id.
fn collect_findings(
    analyzed: &[AnalyzedFile],
    cfg: &Config,
    context: &RepoContext,
) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut commented = commented_out_code::Checker::new();
    for file in analyzed.iter().filter(|file| !file.exempt) {
        let (block_findings, consumed) = commented.run(file, cfg);
        findings.extend(block_findings);
        findings.extend(outside_channel::run_excluding(file, cfg, &consumed));
        findings.extend(doc_cap::run(file, cfg));
        findings.extend(marker::run(file, cfg, context));
        findings.extend(header::run(file, cfg));
        findings.extend(suppression::run(file, cfg));
    }
    findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.check.id().cmp(right.check.id()))
    });
    disambiguate_duplicate_ids(&mut findings);
    smell::classify(analyzed, &mut findings, cfg, context);
    findings
}

/// Make every finding id unique within the report.
///
/// The base identity (`path::symbol-path::content-hash`) is shared by
/// findings whose nodes are identical in every line-independent respect, so
/// a second and later occurrence in report order appends `#<n>`. Unique
/// findings keep their bare identity, which keeps every existing baseline
/// entry valid; the discriminator never depends on line numbers.
fn disambiguate_duplicate_ids(findings: &mut [Finding]) {
    let mut occurrences: BTreeMap<String, usize> = BTreeMap::new();
    for finding in findings.iter_mut() {
        let count = occurrences.entry(finding.id.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            finding.id = format!("{}#{count}", finding.id);
        }
    }
}

/// Load `sad-baseline.json` and apply adoption or suppression, then publish
/// the new-finding count in `stats`.
///
/// Adoption refuses to grow an existing baseline (the new id set must be a
/// subset), then evaluates against the just-written baseline so every adopted
/// finding is grandfathered and the run passes. A plain run suppresses
/// baselined ids and keeps only genuinely new findings; leftover baseline ids
/// the tree no longer produces are simply ignored, and shrinking happens on
/// the next `--adopt`.
fn apply_baseline(
    repo: &Path,
    mut report: CheckReport,
    adopt: bool,
) -> Result<CheckReport, GateFailure> {
    let path = repo.join(baseline::BASELINE_FILE);
    let existing = Baseline::load(&path).map_err(GateFailure::Baseline)?;
    let current_ids: Vec<String> = report
        .findings
        .iter()
        .map(|finding| finding.id.clone())
        .collect();

    if adopt {
        return adopt_baseline(&path, existing.as_ref(), report, &current_ids);
    }
    if let Some(existing) = existing.as_ref() {
        report = suppress_baselined(existing, report);
    }

    // `stats.findings` is the contract's count of *new* findings; suppressed
    // grandfathered ids are not new debt for this run.
    report.stats.findings = report.findings.len();
    Ok(report)
}

/// Write the current ids as the baseline, refusing to grow an existing one.
///
/// The just-written baseline covers every current finding, so the returned
/// report carries none and passes. Re-adopting an unchanged id set keeps its
/// `adopted_at`, so repeating adoption is a no-op on the document.
fn adopt_baseline(
    path: &Path,
    existing: Option<&Baseline>,
    mut report: CheckReport,
    current_ids: &[String],
) -> Result<CheckReport, GateFailure> {
    if let Some(existing) = existing
        && !existing.needs_migration()
    {
        let growth = absent_ids(current_ids, &existing.finding_ids);
        if !growth.is_empty() {
            return Err(GateFailure::Growth {
                message: growth_message(path, &growth),
            });
        }
    }
    Baseline::persist(path, current_ids, existing).map_err(GateFailure::Baseline)?;
    report.findings.clear();
    report.stats.findings = 0;
    Ok(report)
}

/// Drop grandfathered findings from the report.
///
/// A baseline id with no current finding is not an error: the tree may have
/// been fixed without re-adopting, and the next `--adopt` shrinks the file.
/// `stats.findings_suppressed` counts exactly the grandfathered findings this
/// run withheld.
fn suppress_baselined(existing: &Baseline, mut report: CheckReport) -> CheckReport {
    let grandfathered: BTreeSet<&str> = existing.finding_ids.iter().map(String::as_str).collect();
    let findings_before_split = report.findings.len();
    report
        .findings
        .retain(|finding| !grandfathered.contains(finding.id.as_str()));
    report.stats.findings_suppressed = findings_before_split - report.findings.len();
    report
}

/// Ids in `ids` that `present` does not contain, sorted and deduped.
fn absent_ids(ids: &[String], present: &[String]) -> Vec<String> {
    let known: BTreeSet<&str> = present.iter().map(String::as_str).collect();
    ids.iter()
        .filter(|id| !known.contains(id.as_str()))
        .cloned()
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}
