//! Purpose: flag traceability markers (`see <id>`) whose id no available
//! ledger contains — the `unresolvable_marker` anti-fabrication check.
//! Responsibilities: take the whole leading token after the configured
//! `marker_prefix` from every marker-class node (trailing `,` `.` `;` `:`
//! trimmed), leave ids that a consulted ledger knows alone, skip judgment
//! entirely when no ledger that could own a beads-shaped id is available, and
//! emit one `unresolvable_marker` finding per unresolved marker anchored to
//! the cite's stable identity.
//! Rationale: a marker is a promise that an id exists somewhere. Attribution
//! already applied `marker_pattern` when it classified the node, so this
//! check reads the cited token instead of re-matching that pattern: a regex
//! match would truncate `sad-lcy-extra` to its first hyphenated segment and
//! then judge an id the author never wrote. The beads ledger is the source
//! that owns beads-shaped ids, so an absent beads source (including an
//! openspec-only run) must no-op rather than flag every marker in the
//! repository; the check stays a pure function of the analyzed file, config,
//! and repo ledgers — no filesystem or git access.

use crate::checks::RepoContext;
use crate::config::Config;
use crate::model::{AnalyzedFile, AttributionClass, Finding, FindingKind, Severity, finding_id};
use crate::remediation;

/// Punctuation that separates a cited id from the rest of its sentence.
const TRAILING_PUNCTUATION: [char; 4] = [',', '.', ';', ':'];

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

    let mut findings = Vec::new();
    for node in &file.nodes {
        if node.class != AttributionClass::Marker {
            continue;
        }
        let Some(id) = lead_marker_id(&node.body, &cfg.marker_prefix) else {
            continue;
        };
        if ctx.resolves(id) {
            continue;
        }
        findings.push(Finding {
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
        });
    }
    findings
}

/// The ledgers consulted for an id in this run, named for the diagnostic.
fn consulted_ledgers(ctx: &RepoContext) -> &'static str {
    if ctx.openspec_available {
        "beads issue or openspec spec"
    } else {
        "beads issue"
    }
}

/// The cited id of a marker body: the whole leading token after the
/// configured `marker_prefix`, borrowed from `body` so resolution needs no
/// allocation, with sentence punctuation trimmed.
///
/// Attribution already required the prefix followed by a `marker_pattern`
/// match, so a marker-class body yields its cite here; returning the token
/// rather than the regex match keeps a hyphenated id whole.
fn lead_marker_id<'a>(body: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = body.trim_start().strip_prefix(prefix)?;
    let token = rest.split_whitespace().next()?;
    let id = token.trim_end_matches(TRAILING_PUNCTUATION);
    (!id.is_empty()).then_some(id)
}
