//! Purpose: regression tests for the Rule-of-5 review findings on the
//! `missing_header` and `unresolvable_marker` checks.
//! Responsibilities: pin multi-line first-block grouping, keyword token
//! boundaries, whole hyphenated marker ids, the openspec positive resolver,
//! empty-file totality, licence banners above the header, and the
//! keyword-only header predicate.
//! Rationale: each test names one review finding, so a corrective fix cannot
//! silently trade one edge for another; every scenario runs through the real
//! extractor so the tests cannot drift from channel attribution.

use std::path::{Path, PathBuf};

use sad::checks::{RepoContext, header, marker};
use sad::config::Config;
use sad::extract::analyze_file;
use sad::model::{AnalyzedFile, AttributionClass, CommentNode, finding_id};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/checks/marker_header/edges")
        .join(name)
}

/// Analyses an edge-case fixture on disk exactly as the gate would.
#[track_caller]
fn analyze(name: &str, cfg: &Config) -> AnalyzedFile {
    let path = fixture_path(name);
    let rel_path = format!("tests/fixtures/checks/marker_header/edges/{name}");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {} must be readable: {err}", path.display()));
    analyze_file(&path, &rel_path, &source, cfg)
}

/// Config with the dogfood header gate on and every other default intact.
fn header_config() -> Config {
    Config {
        require_header: true,
        ..Config::default()
    }
}

/// Context whose beads ledger exists and contains exactly `ids`.
fn beads_context(ids: &[&str]) -> RepoContext {
    RepoContext {
        beads_ids: ids.iter().map(|id| (*id).to_string()).collect(),
        beads_available: true,
        ..RepoContext::empty()
    }
}

/// Context whose openspec specs directory exists and declares exactly `ids`.
fn openspec_context(ids: &[&str]) -> RepoContext {
    RepoContext {
        openspec_ids: ids.iter().map(|id| (*id).to_string()).collect(),
        openspec_available: true,
        ..RepoContext::empty()
    }
}

#[track_caller]
fn node_containing<'a>(file: &'a AnalyzedFile, needle: &str) -> &'a CommentNode {
    file.nodes
        .iter()
        .find(|node| node.body.contains(needle))
        .unwrap_or_else(|| {
            panic!(
                "{} must carry a comment body containing {needle:?}",
                file.path
            )
        })
}

/// The fixture's single marker node, so marker tests cannot pass on a prose node.
#[track_caller]
fn only_marker_node(file: &AnalyzedFile) -> &CommentNode {
    let markers: Vec<&CommentNode> = file
        .nodes
        .iter()
        .filter(|node| node.class == AttributionClass::Marker)
        .collect();
    assert_eq!(
        markers.len(),
        1,
        "{} must carry exactly one marker node",
        file.path
    );
    markers[0]
}

// CORR-001: a multi-line first block plus its adjacent line comment is one header unit.
#[test]
fn multiline_first_block_with_adjacent_line_comment_passes() {
    let cfg = header_config();
    let file = analyze("header_multiline_ok.rs", &cfg);

    let findings = header::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        0,
        "the block's later lines and the following comment form one first block, got {findings:?}"
    );
}

// CORR-001: the multi-line grouping still fails when a keyword is genuinely absent.
#[test]
fn multiline_first_block_without_rationale_still_fails() {
    let cfg = header_config();
    let file = analyze("header_multiline_missing.rs", &cfg);

    let findings = header::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        1,
        "a grouped first block missing `Rationale` must fail exactly once"
    );
    let finding = &findings[0];
    assert_eq!(finding.check.id(), "missing_header");
    assert_eq!(finding.path, file.path);
    assert_eq!(finding.symbol_path, "-", "a header finding is file-scoped");
    assert!(
        finding.message.contains("Rationale"),
        "the diagnostic must name the absent keyword, got {:?}",
        finding.message
    );
    assert_eq!(
        finding.line, 1,
        "the finding is anchored to the first block's start"
    );
}

// EDGE-001: keyword-shaped tokens with a suffix do not satisfy the required keywords.
#[test]
fn keyword_prefix_does_not_satisfy_the_required_keywords() {
    let cfg = header_config();
    let file = analyze("keyword_boundary.rs", &cfg);

    let findings = header::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        1,
        "`Purposeful`/`ResponsibilitiesMap`/`RationaleFree` must not count as the keywords"
    );
    assert_eq!(findings[0].check.id(), "missing_header");
    assert!(
        findings[0].message.contains("Purpose"),
        "the diagnostic must name `Purpose` as missing, got {:?}",
        findings[0].message
    );
}

// EDGE-001: the exact keywords remain the passing control for the boundary test.
#[test]
fn exact_keyword_block_passes_and_control_is_stable() {
    let cfg = header_config();
    let file = analyze("keyword_control.rs", &cfg);

    let findings = header::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        0,
        "the exact three keywords must satisfy the header check, got {findings:?}"
    );
}

// CORR-002: a two-hyphen marker id resolves as the whole id, not its first segment.
#[test]
fn multi_hyphen_marker_resolves_against_its_full_id() {
    let cfg = Config::default();
    let file = analyze("marker_multi_hyphen_resolvable.rs", &cfg);
    let node = only_marker_node(&file);
    assert!(
        node.body.contains("sad-lcy-extra"),
        "the fixture must cite the two-hyphen id"
    );

    let ctx = beads_context(&["sad-lcy-extra"]);
    let findings = marker::run(&file, &cfg, &ctx);
    assert_eq!(
        findings.len(),
        0,
        "`sad-lcy-extra` is in the ledger, so truncating it must not fabricate a finding"
    );
}

// CORR-002: an unresolvable two-hyphen marker reports the full fabricated id.
#[test]
fn fabricated_multi_hyphen_marker_reports_the_full_id() {
    let cfg = Config::default();
    let file = analyze("marker_multi_hyphen_fabricated.rs", &cfg);
    let node = only_marker_node(&file);

    let ctx = beads_context(&["sad-lcy"]);
    assert!(
        ctx.resolves("sad-lcy"),
        "the truncated segment is what makes the bug look resolvable"
    );
    let findings = marker::run(&file, &cfg, &ctx);
    assert_eq!(
        findings.len(),
        1,
        "`sad-lcy-extra` is not in the ledger and must be reported"
    );
    let finding = &findings[0];
    assert_eq!(finding.check.id(), "unresolvable_marker");
    assert!(
        finding.message.contains("`sad-lcy-extra`"),
        "the diagnostic must name the full id, got {:?}",
        finding.message
    );
    assert!(
        !finding.message.contains("`sad-lcy`"),
        "the diagnostic must not name the truncated id, got {:?}",
        finding.message
    );
    assert_eq!(
        finding.id,
        finding_id(&file.path, &node.symbol_path, &node.body),
        "the finding id is the stable id of the cited comment"
    );
    assert_eq!(finding.line, node.line);
    assert_eq!(finding.symbol_path, node.symbol_path);
}

// EDGE-003: an openspec id resolves a marker positively instead of being flagged.
#[test]
fn openspec_resolver_satisfies_a_cited_marker() {
    let cfg = Config::default();
    let file = analyze("marker_openspec.rs", &cfg);
    let node = only_marker_node(&file);
    assert!(
        node.body.contains("see comment-gate"),
        "the fixture must cite the openspec capability name"
    );

    let ctx = openspec_context(&["comment-gate"]);
    assert!(
        ctx.resolves("comment-gate"),
        "`comment-gate` matches the default marker pattern and the openspec ledger"
    );
    let findings = marker::run(&file, &cfg, &ctx);
    assert_eq!(
        findings.len(),
        0,
        "a marker resolving against openspec must not be flagged, got {findings:?}"
    );
}

// EDGE-003: an empty file is a total-function case, not a panic.
#[test]
fn empty_file_reports_one_missing_header_and_no_marker_findings() {
    let cfg = header_config();
    let file = analyze("empty.rs", &cfg);
    assert!(file.nodes.is_empty(), "zero bytes yield no comment nodes");

    let findings = header::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        1,
        "an empty first block lacks every keyword, so exactly one finding is due"
    );
    assert_eq!(findings[0].check.id(), "missing_header");
    assert_eq!(findings[0].path, file.path);
    assert_eq!(
        findings[0].symbol_path, "-",
        "an empty file's header finding is still file-scoped"
    );

    let ctx = RepoContext::empty();
    assert_eq!(
        marker::run(&file, &cfg, &ctx).len(),
        0,
        "an empty file has no marker nodes"
    );
}

// EDGE-002: the header block may follow a licence banner in the leading comment area.
#[test]
fn licence_banner_above_the_header_still_satisfies_the_check() {
    // Contract: `missing_header` scans the leading run of comment nodes and
    // skips Exemption-class nodes, so the header is the first non-exempt
    // comment block, not necessarily the file's first physical block.
    let cfg = header_config();
    let file = analyze("licence_then_header.rs", &cfg);

    let banner = node_containing(&file, "SPDX-License-Identifier");
    assert_eq!(
        banner.class,
        AttributionClass::Exemption,
        "the SPDX banner must be Exemption-class for the skip rule to apply"
    );
    assert!(
        !file.header,
        "the licence banner occupies the first node position, so the positional header channel is unset here"
    );

    let findings = header::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        0,
        "the keyword block after the licence banner must satisfy the check, got {findings:?}"
    );
}

// DRAFT-001: the header channel alone is not a header — the keywords decide.
#[test]
fn header_channel_without_keywords_still_reports_missing_header() {
    let cfg = header_config();
    let file = analyze("header_true_no_keywords.rs", &cfg);
    assert!(
        file.header,
        "the leading `//!` block sets the positional header channel"
    );

    let findings = header::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        1,
        "header position without keywords must fail exactly once"
    );
    assert_eq!(findings[0].check.id(), "missing_header");
    assert_eq!(findings[0].symbol_path, "-");
}

// DRAFT-001: a keyword block after `use` statements satisfies the check without the header channel.
#[test]
fn keyword_block_after_use_statements_satisfies_the_check() {
    let cfg = header_config();
    let file = analyze("header_after_code.rs", &cfg);
    assert!(
        !file.header,
        "the file starts with a use declaration, so the positional header channel is unset"
    );

    let findings = header::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        0,
        "the check is keyword-based, not header-channel-based, got {findings:?}"
    );
}
