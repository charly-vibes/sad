//! Purpose: flag traceability markers (`see <id>`) whose id no available
//! ledger contains — the `unresolvable_marker` anti-fabrication check.
//! Responsibilities: extract the cited id from every marker-class node with
//! the same attribution extractor that classified it, so judgment reads the
//! id the author cited, leave ids that a consulted ledger knows alone, skip
//! judgment
//! entirely when no ledger that could own a beads-shaped id is available, and
//! emit one `unresolvable_marker` finding per unresolved marker anchored to
//! the cite's stable identity.
//! Rationale: a marker is a promise that an id exists somewhere. Attribution
//! classified the node with [`leading_marker_id`], whose widening keeps
//! `sad-lcy-extra` whole, so the check extracts the id with that same
//! extractor: one id definition, and judgment can never diverge from
//! classification. The beads ledger is the source
//! that owns beads-shaped ids, so an absent beads source (including an
//! openspec-only run) must no-op rather than flag every marker in the
//! repository; the check stays a pure function of the analyzed file, config,
//! and repo ledgers — no filesystem or git access.

use regex::Regex;

use crate::attribution::leading_marker_id;
use crate::checks::RepoContext;
use crate::config::Config;
use crate::model::{
    AnalyzedFile, AttributionClass, CommentNode, Finding, FindingKind, Severity, finding_id,
};
use crate::remediation;

/// Judge every marker-class node in `file` against the available ledgers.
///
/// A disabled toggle yields an empty vector. Beads-shaped ids are judged only
/// while the beads ledger exists: the openspec directory declares capability
/// names that cannot prove a fabricated id, so its presence alone never turns
/// a marker into a finding. A file with no marker nodes yields nothing.
pub fn run(file: &AnalyzedFile, cfg: &Config, ctx: &RepoContext) -> Vec<Finding> {
    if !cfg.checks.unresolvable_marker {
        return Vec::new();
    }
    if !ctx.beads_available {
        return Vec::new();
    }
    // The early return above means beads is always consulted here; name the
    // ledgers that actually ran so the diagnostic cannot cite a check that
    // never happened.
    let ledgers = consulted_ledgers(ctx);
    // WHY: an invalid pattern classifies no marker nodes, so an unusable matcher sees none.
    let pattern = Regex::new(&cfg.marker_pattern).ok();

    let mut findings = Vec::new();
    for node in &file.nodes {
        if node.class != AttributionClass::Marker {
            continue;
        }
        let Some(id) = cited_id(node, cfg, &pattern) else {
            continue;
        };
        if ctx.resolves(id) {
            continue;
        }
        findings.push(unresolvable_finding(file, node, id, ledgers));
    }
    findings
}

/// The cited id of a marker-class node, extracted with the same attribution
/// extractor that classified the node, so judgment reads what was cited.
fn cited_id<'a>(node: &'a CommentNode, cfg: &Config, pattern: &Option<Regex>) -> Option<&'a str> {
    pattern
        .as_ref()
        .and_then(|regex| leading_marker_id(&node.body, &cfg.marker_prefix, regex))
}

/// The `unresolvable_marker` finding for a marker node whose cited id no
/// available ledger knows.
fn unresolvable_finding(
    file: &AnalyzedFile,
    node: &CommentNode,
    id: &str,
    ledgers: &str,
) -> Finding {
    Finding {
        check: FindingKind::UnresolvableMarker,
        id: finding_id(&file.path, &node.symbol_path, &node.body),
        path: file.path.clone(),
        line: node.line,
        symbol_path: node.symbol_path.clone(),
        message: format!("traceability marker cites `{id}`, which resolves to no {ledgers}"),
        text: node.body.clone(),
        severity: Severity::Error,
        smell: None,
        remediation: remediation::for_kind(FindingKind::UnresolvableMarker, &file.path),
    }
}

/// The ledgers consulted for an id in this run, named for the diagnostic.
fn consulted_ledgers(ctx: &RepoContext) -> &'static str {
    if ctx.openspec_available {
        "beads issue or openspec spec"
    } else {
        "beads issue"
    }
}
