//! Purpose: contract tests for the `unresolvable_marker` and `missing_header`
//! gate checks (`sad::checks::marker` + `sad::checks::header`).
//! Responsibilities: pin marker resolution against the beads ledger, resolver
//! source no-ops, the marker prefix/pattern knobs, the unresolvable-marker
//! toggle, order-free header detection, partial-header failure, the header
//! gate/toggle, and first-block-only inspection.
//! Rationale: these two checks are the anti-fabrication and dogfood-header
//! guarantees; each scenario is driven through the real extractor so the
//! tests cannot drift from channel attribution.

use std::path::{Path, PathBuf};

use sad::checks::{RepoContext, header, marker};
use sad::config::Config;
use sad::extract::analyze_file;
use sad::model::{AnalyzedFile, AttributionClass, CommentNode, finding_id};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/checks/marker_header")
        .join(name)
}

/// Analyses a fixture on disk exactly as the gate would, with the given config.
#[track_caller]
fn analyze(name: &str, cfg: &Config) -> AnalyzedFile {
    let path = fixture_path(name);
    let rel_path = format!("tests/fixtures/checks/marker_header/{name}");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {} must be readable: {err}", path.display()));
    analyze_file(&path, &rel_path, &source, cfg)
}

/// Context whose beads ledger exists and contains exactly `ids`.
fn beads_context(ids: &[&str]) -> RepoContext {
    RepoContext {
        beads_ids: ids.iter().map(|id| (*id).to_string()).collect(),
        beads_available: true,
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

#[track_caller]
fn assert_file_scoped_finding_id(id: &str, path: &str) {
    let parts: Vec<&str> = id.split("::").collect();
    assert_eq!(
        parts.len(),
        3,
        "finding id `{id}` must be path::symbol::hash"
    );
    assert_eq!(parts[0], path, "the path component is the analyzed file");
    assert_eq!(
        parts[1], "-",
        "file-scoped findings use `-` as the symbol component"
    );
    assert_eq!(parts[2].len(), 16, "the hash component is 64-bit hex");
    assert!(
        parts[2].chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
        "the hash component is lowercase hex, got {:?}",
        parts[2]
    );
}

// spec: Traceability markers resolve / Resolvable ticket ID.
#[test]
fn resolvable_marker_emits_no_findings() {
    let cfg = Config::default();
    let file = analyze("resolvable.rs", &cfg);
    let node = node_containing(&file, "see sad-lcy");
    assert_eq!(
        node.class,
        AttributionClass::Marker,
        "the prefixed comment is a marker node"
    );

    let ctx = beads_context(&["sad-lcy"]);
    assert!(ctx.resolves("sad-lcy"), "the fixture id must resolve");
    let findings = marker::run(&file, &cfg, &ctx);
    assert_eq!(
        findings.len(),
        0,
        "a marker with a resolvable id must not be flagged"
    );
}

// spec: Traceability markers resolve / Fabricated ID.
#[test]
fn fabricated_marker_emits_one_unresolvable_marker() {
    let cfg = Config::default();
    let file = analyze("fabricated.rs", &cfg);
    let node = only_marker_node(&file);
    assert!(
        node.body.contains("sad-zzz"),
        "the marker body cites the fabricated id"
    );

    let ctx = beads_context(&["sad-lcy"]);
    let findings = marker::run(&file, &cfg, &ctx);
    assert_eq!(findings.len(), 1, "exactly one finding for one marker");
    let finding = &findings[0];
    assert_eq!(finding.check.id(), "unresolvable_marker");
    assert_eq!(
        finding.id,
        finding_id(&file.path, &node.symbol_path, &node.body),
        "the finding id is the stable id of the cited comment"
    );
    assert!(
        finding.message.contains("sad-zzz"),
        "the diagnostic must name the fabricated id, got {:?}",
        finding.message
    );
    assert_eq!(finding.path, file.path);
    assert_eq!(finding.line, node.line);
    assert_eq!(finding.symbol_path, node.symbol_path);
}

// spec: Traceability markers resolve / each resolver no-ops while its source is absent.
#[test]
fn fabricated_marker_is_ignored_when_its_resolver_source_is_absent() {
    let cfg = Config::default();
    let file = analyze("fabricated.rs", &cfg);
    only_marker_node(&file);

    let no_sources = RepoContext::empty();
    assert_eq!(
        marker::run(&file, &cfg, &no_sources).len(),
        0,
        "with both sources absent, marker resolution is skipped entirely"
    );

    let openspec_only = RepoContext {
        openspec_available: true,
        ..RepoContext::empty()
    };
    assert_eq!(
        marker::run(&file, &cfg, &openspec_only).len(),
        0,
        "an absent beads source no-ops for a beads-shaped id"
    );
}

// spec: Traceability markers resolve / Version-number lookalike is not a marker.
#[test]
fn version_lookalike_is_prose_and_not_flagged() {
    let cfg = Config::default();
    let file = analyze("lookalike.rs", &cfg);
    let node = node_containing(&file, "serde-1");
    assert_eq!(
        node.class,
        AttributionClass::Prose,
        "an id-shaped token without the marker prefix stays prose"
    );

    let ctx = beads_context(&["sad-lcy"]);
    assert_eq!(
        marker::run(&file, &cfg, &ctx).len(),
        0,
        "the marker check must not judge prose nodes"
    );
}

// spec: Traceability markers resolve / leading token is the configured marker prefix.
#[test]
fn custom_prefix_replaces_the_default_marker_prefix() {
    let cfg = Config {
        marker_prefix: "ticket".to_string(),
        ..Config::default()
    };
    let file = analyze("resolvable.rs", &cfg);

    let default_prefix_node = node_containing(&file, "see sad-lcy");
    assert_eq!(
        default_prefix_node.class,
        AttributionClass::Prose,
        "the default prefix is not the configured prefix"
    );
    let custom_prefix_node = node_containing(&file, "ticket sad-lcy");
    assert_eq!(
        custom_prefix_node.class,
        AttributionClass::Marker,
        "the configured prefix routes the comment to the marker channel"
    );

    let ctx = beads_context(&["sad-lcy"]);
    assert_eq!(
        marker::run(&file, &cfg, &ctx).len(),
        0,
        "the custom-prefix marker resolves, and the other comment is prose"
    );
}

// spec: Traceability markers resolve / marker_pattern gates which ids are markers.
#[test]
fn narrowed_marker_pattern_turns_the_marker_back_into_prose() {
    let cfg = Config {
        marker_pattern: r"\bissue-[0-9]+\b".to_string(),
        ..Config::default()
    };
    let file = analyze("resolvable.rs", &cfg);

    let node = node_containing(&file, "see sad-lcy");
    assert_eq!(
        node.class,
        AttributionClass::Prose,
        "an id outside the configured pattern is not a marker"
    );

    let ctx = beads_context(&["sad-lcy"]);
    assert_eq!(
        marker::run(&file, &cfg, &ctx).len(),
        0,
        "the marker check must not judge prose nodes"
    );
}

// spec: Traceability markers resolve / per-check toggle disables unresolvable_marker.
#[test]
fn unresolvable_marker_toggle_disables_findings() {
    let mut cfg = Config::default();
    cfg.checks.unresolvable_marker = false;
    let file = analyze("fabricated.rs", &cfg);
    only_marker_node(&file);

    let ctx = beads_context(&["sad-lcy"]);
    assert_eq!(
        marker::run(&file, &cfg, &ctx).len(),
        0,
        "the disabled check must emit nothing"
    );
}

// spec: File header requirement is dogfood-default / Header present, keywords out of order.
#[test]
fn complete_header_passes_regardless_of_keyword_order() {
    let cfg = Config {
        require_header: true,
        ..Config::default()
    };
    let file = analyze("header_ordered.rs", &cfg);

    let findings = header::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        0,
        "a first block naming all three keywords passes in any order"
    );
}

// spec: File header requirement is dogfood-default / Partial header fails.
#[test]
fn partial_header_emits_missing_header_with_file_scoped_id() {
    let cfg = Config {
        require_header: true,
        ..Config::default()
    };
    let file = analyze("header_partial.rs", &cfg);

    let findings = header::run(&file, &cfg);
    assert_eq!(findings.len(), 1, "a Purpose-only first block must fail");
    let finding = &findings[0];
    assert_eq!(finding.check.id(), "missing_header");
    assert_eq!(finding.path, file.path);
    assert_eq!(finding.symbol_path, "-", "a header finding is file-scoped");
    assert_file_scoped_finding_id(&finding.id, &file.path);
}

// spec: File header requirement is dogfood-default / Check disabled.
#[test]
fn header_check_is_off_by_default() {
    let cfg = Config::default();
    assert!(
        !cfg.require_header,
        "require_header defaults off outside sad's own repo"
    );
    let file = analyze("header_partial.rs", &cfg);

    assert_eq!(
        header::run(&file, &cfg).len(),
        0,
        "a disabled header gate must emit nothing"
    );
}

// spec: File header requirement is dogfood-default / per-check toggle disables missing_header.
#[test]
fn missing_header_toggle_disables_findings() {
    let mut cfg = Config {
        require_header: true,
        ..Config::default()
    };
    cfg.checks.missing_header = false;
    let file = analyze("header_partial.rs", &cfg);

    assert_eq!(
        header::run(&file, &cfg).len(),
        0,
        "the disabled check must emit nothing even when the gate is on"
    );
}

// spec: File header requirement is dogfood-default / only the first block is inspected.
#[test]
fn header_keywords_later_in_the_file_do_not_satisfy_the_first_block() {
    let cfg = Config {
        require_header: true,
        ..Config::default()
    };
    let file = analyze("header_late.rs", &cfg);

    let findings = header::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        1,
        "valid keywords outside the first block do not rescue the file"
    );
    assert_eq!(findings[0].check.id(), "missing_header");
    assert_eq!(findings[0].symbol_path, "-");
}
