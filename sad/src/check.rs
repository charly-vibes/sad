//! Purpose: run the whole documentation gate over a repository tree — file
//! discovery, exclusion, commit scoping, extraction, every enabled check, the
//! baseline split, and the stats and suppression-debt the envelope reports.
//! Responsibilities: collect supported source files deterministically, apply
//! `exclude` globs and commit-attribution scope, build the marker resolver
//! context from the beads ledger and openspec specs, run each check on
//! non-exempt files, order findings by path, line, and check id, then load
//! `sad-baseline.json` to grandfather adopted ids, adopt on request, and
//! enforce the shrink-only invariant.
//! Rationale: the envelope is only trustworthy if its inputs are reproducible,
//! so discovery is a pure tree walk with no clock, network, or parallelism and
//! every collection is sorted before it reaches the report. Unreadable files
//! and directories are skipped deterministically (the gate reports on what it
//! can parse; an I/O failure is not a documentation finding and must not make
//! the same tree produce different results on different machines).

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::baseline::{self, Baseline, BaselineError};
use crate::checks::{RepoContext, doc_cap, header, marker, outside_channel, suppression};
use crate::config::Config;
use crate::exemptions;
use crate::extract;
use crate::model::{AnalyzedFile, AttributionClass, Finding, SuppressionEntry};
use crate::scope;

/// Extensions the gate understands (matching [`extract::language_for_path`]).
const SOURCE_EXTENSIONS: [&str; 3] = ["rs", "py", "ts"];

/// Directory names never traversed: VCS metadata, build output, the rendered
/// book, the issue ledger, and dependency trees. Skipping them keeps the walk
/// finite and independent of what tooling last ran in the tree.
const SKIPPED_DIRECTORIES: [&str; 5] = [".git", "target", "book", ".beads", "node_modules"];

/// Beads ledger path, relative to the repository root.
const BEADS_LEDGER: &str = ".beads/issues.jsonl";

/// OpenSpec specs directory, relative to the repository root.
const OPENSPEC_SPECS: &str = "openspec/specs";

/// OpenSpec change deltas, relative to the repository root — each change may
/// declare spec/capability ids (for example `comment-gate`) before they are
/// synced into `openspec/specs/`.
const OPENSPEC_CHANGES: &str = "openspec/changes";

/// Machine-readable body of the `sad check` envelope.
///
/// Field names are the contract: `findings`, `suppression_debt`, and `stats`
/// with one count per channel plus the run totals.
#[derive(Debug, Clone, Serialize)]
pub struct CheckReport {
    pub findings: Vec<Finding>,
    pub suppression_debt: Vec<SuppressionEntry>,
    pub stats: CheckStats,
}

/// Counts describing what one gate run looked at.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CheckStats {
    /// Analysed files that were not whole-file exempt.
    pub files_analyzed: usize,
    /// Analysed files the extractor marked exempt.
    pub files_exempt: usize,
    /// Exemption-class comment nodes across the analysed tree.
    pub exempt_nodes: usize,
    pub comments_header: usize,
    pub comments_doc: usize,
    pub comments_marker: usize,
    pub comments_suppression: usize,
    pub comments_prose: usize,
    pub suppression_debt: usize,
    pub findings: usize,
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
    /// The baseline would grow (adoption) or is larger than the current
    /// finding set (stale ids), violating the shrink-only invariant.
    Growth { message: String },
}

impl GateFailure {
    /// Stable error code for the envelope.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Baseline(_) => "baseline_error",
            Self::Growth { .. } => "baseline_growth",
        }
    }
}

impl fmt::Display for GateFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Baseline(error) => write!(f, "{error}"),
            Self::Growth { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for GateFailure {}

/// Analyse `repo`, apply the baseline gate, and build the check report.
///
/// `all` bypasses commit-attribution scope; configuration toggles and caps are
/// honoured by the individual checks. With `adopt`, the current findings are
/// persisted as the baseline (bounded by the shrink-only invariant) and the
/// run reports a clean pass. Otherwise ids present in `sad-baseline.json` are
/// suppressed, while genuinely new findings are reported and fail the run.
pub fn run(repo: &Path, cfg: &Config, all: bool, adopt: bool) -> Result<CheckReport, GateFailure> {
    let discovered = discover_files(repo);
    let candidates: Vec<String> = discovered
        .into_iter()
        .filter(|file| !exemptions::is_excluded(file, cfg))
        .collect();
    let selected = scope::select(repo, &candidates, cfg, all);
    let context = repo_context(repo);

    let mut analyzed: Vec<AnalyzedFile> = Vec::new();
    for rel_path in &selected {
        let Ok(source) = fs::read_to_string(repo.join(rel_path)) else {
            // Unreadable or non-UTF-8 source: skip it rather than invent a
            // finding or let the machine's I/O state change the report.
            continue;
        };
        analyzed.push(extract::analyze_file(
            Path::new(rel_path),
            rel_path,
            &source,
            cfg,
        ));
    }

    let mut findings: Vec<Finding> = Vec::new();
    let mut debt: Vec<SuppressionEntry> = Vec::new();
    for file in &analyzed {
        debt.extend(suppression::entries(file));
        if file.exempt {
            // Exempt files are observable in the stats but never finding
            // eligible.
            continue;
        }
        findings.extend(outside_channel::run(file, cfg));
        findings.extend(doc_cap::run(file, cfg));
        findings.extend(marker::run(file, cfg, &context));
        findings.extend(header::run(file, cfg));
        findings.extend(suppression::run(file, cfg));
    }
    findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.check.id().cmp(right.check.id()))
    });

    let exemption_summary = exemptions::summary(&analyzed);
    let stats = CheckStats {
        files_analyzed: analyzed.len() - exemption_summary.files_exempt,
        files_exempt: exemption_summary.files_exempt,
        exempt_nodes: exemption_summary.nodes_exempt,
        comments_header: count_class(&analyzed, AttributionClass::Header),
        comments_doc: count_class(&analyzed, AttributionClass::Doc),
        comments_marker: count_class(&analyzed, AttributionClass::Marker),
        comments_suppression: count_class(&analyzed, AttributionClass::Suppression),
        comments_prose: count_class(&analyzed, AttributionClass::Prose),
        suppression_debt: debt.len(),
        findings: findings.len(),
    };

    let report = CheckReport {
        findings,
        suppression_debt: debt,
        stats,
    };
    apply_baseline(repo, report, adopt)
}

/// Load `sad-baseline.json`, apply adoption or suppression, and return the
/// report the envelope should carry.
///
/// Adoption refuses to grow an existing baseline (the new id set must be a
/// subset), then evaluates against the just-written baseline so every adopted
/// finding is grandfathered and the run passes. A plain run suppresses
/// baselined ids and keeps only genuinely new findings; when nothing new is
/// reported but the baseline still carries ids the tree no longer produces,
/// that staleness is a shrink-only gate failure rather than a prose finding.
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
        if let Some(existing) = &existing {
            let known: BTreeSet<&str> = existing.finding_ids.iter().map(String::as_str).collect();
            let growth: Vec<String> = current_ids
                .iter()
                .filter(|id| !known.contains(id.as_str()))
                .cloned()
                .collect();
            if !growth.is_empty() {
                return Err(GateFailure::Growth {
                    message: growth_message(&path, &growth, GrowthDirection::Adopt),
                });
            }
        }
        Baseline::persist(&path, &current_ids).map_err(GateFailure::Baseline)?;
        // The baseline just written covers every current finding, so this run
        // reports no new findings and passes.
        report.findings.clear();
        report.stats.findings = 0;
        return Ok(report);
    }

    if let Some(existing) = &existing {
        let grandfathered: BTreeSet<&str> =
            existing.finding_ids.iter().map(String::as_str).collect();
        let findings_before_split = report.findings.len();
        report
            .findings
            .retain(|finding| !grandfathered.contains(finding.id.as_str()));

        // Shrink-only: once every remaining finding is grandfathered (there is
        // nothing new to report), the baseline must not carry ids that no
        // longer correspond to a current finding. A wholly clean tree may
        // leave the larger baseline in place until re-adoption shrinks it;
        // new findings are always reported instead of being masked by a stale
        // baseline.
        if findings_before_split > 0 && report.findings.is_empty() {
            let current: BTreeSet<&str> = current_ids.iter().map(String::as_str).collect();
            let stale: Vec<String> = existing
                .finding_ids
                .iter()
                .filter(|id| !current.contains(id.as_str()))
                .cloned()
                .collect();
            if !stale.is_empty() {
                return Err(GateFailure::Growth {
                    message: growth_message(&path, &stale, GrowthDirection::Stale),
                });
            }
        }
    }

    // `stats.findings` is the contract's count of *new* findings; suppressed
    // grandfathered ids are not new debt for this run.
    report.stats.findings = report.findings.len();
    Ok(report)
}

/// Which way the shrink-only invariant was violated.
enum GrowthDirection {
    /// Adoption would add ids that are not in the existing baseline.
    Adopt,
    /// The baseline holds ids that are no longer current findings.
    Stale,
}

/// Human-readable growth failure naming the baseline path and the ids.
fn growth_message(path: &Path, ids: &[String], direction: GrowthDirection) -> String {
    let listing = ids.join(", ");
    match direction {
        GrowthDirection::Adopt => format!(
            "refusing to grow baseline `{}`: {} finding id(s) are not in the existing baseline \
             ({listing}); the baseline is shrink-only, so fix those findings instead of \
             adopting them",
            path.display(),
            ids.len()
        ),
        GrowthDirection::Stale => format!(
            "baseline `{}` is stale: {} id(s) are not present in the current findings \
             ({listing}), so the baseline is larger than the tree; re-adopt the smaller set \
             (shrink) or fix the tree",
            path.display(),
            ids.len()
        ),
    }
}

/// Every supported source file under `repo`, repo-relative, forward-slashed,
/// and sorted. Unreadable directories and symlinks are skipped.
pub fn discover_files(repo: &Path) -> Vec<String> {
    let mut files = Vec::new();
    walk(repo, repo, &mut files);
    files.sort();
    files
}

fn walk(root: &Path, dir: &Path, files: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            if SKIPPED_DIRECTORIES.contains(&name.as_str()) {
                continue;
            }
            walk(root, &entry.path(), files);
            continue;
        }
        if !file_type.is_file() {
            // Symlinks are neither files nor directories here: following them
            // could loop, so the walk stays inside the real tree.
            continue;
        }
        let is_source = entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| SOURCE_EXTENSIONS.contains(&extension));
        if !is_source {
            continue;
        }
        let path = entry.path();
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let parts: Vec<String> = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        files.push(parts.join("/"));
    }
}

/// Count how many analysed comments were attributed to `class`.
fn count_class(files: &[AnalyzedFile], class: AttributionClass) -> usize {
    files
        .iter()
        .flat_map(|file| file.nodes.iter())
        .filter(|node| node.class == class)
        .count()
}

/// Build the marker resolver context from the repository's ledgers.
///
/// The beads ledger is available when `.beads/issues.jsonl` can be read; every
/// JSONL line is parsed independently and malformed lines are ignored, so one
/// corrupt record cannot hide the rest. The openspec resolver is available when
/// `openspec/specs/` exists; its ids are the spec directory names plus the
/// capability directories declared by pending change deltas
/// (`openspec/changes/*/specs/*`).
fn repo_context(repo: &Path) -> RepoContext {
    let (beads_ids, beads_available) = match fs::read_to_string(repo.join(BEADS_LEDGER)) {
        Ok(text) => (parse_beads_ids(&text), true),
        Err(_) => (BTreeSet::new(), false),
    };

    let specs_dir = repo.join(OPENSPEC_SPECS);
    let openspec_available = specs_dir.is_dir();
    let mut openspec_ids = BTreeSet::new();
    if openspec_available {
        collect_directory_names(&specs_dir, &mut openspec_ids);
        let changes_dir = repo.join(OPENSPEC_CHANGES);
        let Ok(entries) = fs::read_dir(&changes_dir) else {
            return RepoContext {
                beads_ids,
                openspec_ids,
                beads_available,
                openspec_available,
            };
        };
        let mut changes: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        changes.sort();
        for change in changes {
            collect_directory_names(&change.join("specs"), &mut openspec_ids);
        }
    }

    RepoContext {
        beads_ids,
        openspec_ids,
        beads_available,
        openspec_available,
    }
}

/// Parse beads JSONL into the set of declared issue ids, ignoring malformed
/// lines and records without a string `id`.
fn parse_beads_ids(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|record| {
            record
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect()
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
    fn discovery_uses_repo_relative_forward_slashes_and_sorts() {
        let root = std::env::temp_dir().join(format!("sad-check-unit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("nested")).expect("fixture tree is created");
        fs::create_dir_all(root.join("target")).expect("skipped tree is created");
        fs::write(root.join("nested/b.rs"), "fn b() {}").expect("fixture is written");
        fs::write(root.join("a.py"), "").expect("fixture is written");
        fs::write(root.join("target/skip.rs"), "fn skip() {}").expect("fixture is written");

        let files = discover_files(&root);
        assert_eq!(files, vec!["a.py".to_string(), "nested/b.rs".to_string()]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn malformed_beads_lines_do_not_hide_valid_ids() {
        let ids = parse_beads_ids("not json\n{\"id\":\"sad-a\"}\n{\"title\":\"no id\"}\n");
        assert_eq!(ids, BTreeSet::from(["sad-a".to_string()]));
    }
}
