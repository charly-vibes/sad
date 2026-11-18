//! Purpose: contract tests for the prose checks — `comment_outside_channel`
//! (`sad::checks::outside_channel`) and `doc_summary_cap_exceeded`
//! (`sad::checks::doc_cap`).
//! Responsibilities: pin the finding shape for narration comments, silence
//! when a file carries no prose, stable finding ids under line drift, and the
//! doc-summary cap's structured-section pass, two-line fail, non-public parity,
//! marker-prefix laundering immunity, and config knobs.
//! Rationale: both checks are pure functions of an analyzed file plus config;
//! driving them through `analyze_file` keeps the tests on the same structural
//! attributions the gate sees at runtime, with no hand-built nodes to drift.

use std::path::{Path, PathBuf};

use sad::checks::{doc_cap, outside_channel};
use sad::config::{CheckToggles, Config};
use sad::extract::analyze_file;
use sad::model::{AnalyzedFile, AttributionClass, CommentNode, Finding, FindingKind, finding_id};

const NARRATION: &str = include_str!("fixtures/checks/prose/narration.rs");
const TODO: &str = include_str!("fixtures/checks/prose/todo.py");
const CLEAN: &str = include_str!("fixtures/checks/prose/clean.rs");
const STRUCTURED: &str = include_str!("fixtures/checks/prose/doc_structured.rs");
const TWO_LINE: &str = include_str!("fixtures/checks/prose/doc_two_line.rs");
const NONPUBLIC: &str = include_str!("fixtures/checks/prose/nonpublic_docstring.py");
const LAUNDERING: &str = include_str!("fixtures/checks/prose/laundering.py");

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/checks/prose")
        .join(name)
}

/// Analyses a prose fixture exactly as the gate would: source from the fixture
/// file, stable repo-relative path for finding ids.
fn analyze(name: &str, source: &str, cfg: &Config) -> AnalyzedFile {
    let rel_path = format!("tests/fixtures/checks/prose/{name}");
    analyze_file(&fixture_path(name), &rel_path, source, cfg)
}

fn nodes_of(file: &AnalyzedFile, class: AttributionClass) -> Vec<&CommentNode> {
    file.nodes
        .iter()
        .filter(|node| node.class == class)
        .collect()
}

/// The single outside-channel finding a drift variant must produce, so the
/// id comparisons cannot accidentally ride on an empty or duplicated result.
#[track_caller]
fn only_outside_channel(source: &str, cfg: &Config) -> Finding {
    let file = analyze("narration.rs", source, cfg);
    let findings = outside_channel::run(&file, cfg);
    assert_eq!(
        findings.len(),
        1,
        "{} must yield exactly one comment_outside_channel finding",
        file.path
    );
    findings.into_iter().next().expect("length was asserted")
}

// spec: Prose outside channels / Narration comment.
#[test]
fn narration_comment_in_a_rust_fn_body_is_one_finding() {
    let cfg = Config::default();
    let file = analyze("narration.rs", NARRATION, &cfg);

    let prose = nodes_of(&file, AttributionClass::Prose);
    assert_eq!(prose.len(), 1, "the fixture carries exactly one prose node");
    let node = prose[0];
    assert_eq!(
        node.symbol_path, "total",
        "the narration sits inside the `total` fn body"
    );

    let findings = outside_channel::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        1,
        "a single narration comment yields a single finding"
    );
    let finding = &findings[0];
    assert_eq!(finding.check, FindingKind::CommentOutsideChannel);
    assert_eq!(finding.path, file.path);
    assert_eq!(finding.line, node.line);
    assert_eq!(finding.symbol_path, "total");
    assert_eq!(
        finding.id,
        finding_id(&file.path, &node.symbol_path, &node.body),
        "the id is the stable (path, symbol, body) triple"
    );
}

// spec: Prose outside channels / TODO stub.
#[test]
fn module_level_todo_is_file_scoped_prose() {
    let cfg = Config::default();
    let file = analyze("todo.py", TODO, &cfg);

    let prose = nodes_of(&file, AttributionClass::Prose);
    assert_eq!(prose.len(), 1);
    let node = prose[0];
    assert_eq!(node.symbol_path, "-", "outside every item is file-scoped");

    let findings = outside_channel::run(&file, &cfg);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].check, FindingKind::CommentOutsideChannel);
    assert_eq!(findings[0].line, node.line);
    assert_eq!(findings[0].symbol_path, "-");
}

// spec: Prose outside channels / No finding when the file has no prose.
#[test]
fn file_without_prose_has_no_outside_channel_findings() {
    let cfg = Config::default();
    let file = analyze("clean.rs", CLEAN, &cfg);

    assert!(
        nodes_of(&file, AttributionClass::Marker).len() == 1,
        "the fixture must carry a marker node"
    );
    assert!(
        nodes_of(&file, AttributionClass::Suppression).len() == 1,
        "the fixture must carry a suppression node"
    );
    assert!(
        nodes_of(&file, AttributionClass::Prose).is_empty(),
        "the fixture must not carry prose"
    );

    assert!(
        outside_channel::run(&file, &cfg).is_empty(),
        "header, doc, marker, and suppression nodes are all documented channels"
    );
}

// spec: Prose outside channels / The check toggle disables it.
#[test]
fn disabled_toggle_silences_outside_channel() {
    let cfg = Config {
        checks: CheckToggles {
            comment_outside_channel: false,
            ..Config::default().checks
        },
        ..Config::default()
    };
    let file = analyze("narration.rs", NARRATION, &cfg);

    assert_eq!(
        nodes_of(&file, AttributionClass::Prose).len(),
        1,
        "attribution is independent of the toggle"
    );
    assert!(outside_channel::run(&file, &cfg).is_empty());
}

// spec: Prose outside channels / Stable ID under line drift and body/symbol changes.
#[test]
fn finding_id_is_stable_under_line_drift_and_sensitive_to_body_and_symbol() {
    let cfg = Config::default();
    let base = only_outside_channel(NARRATION, &cfg);
    assert_eq!(base.symbol_path, "total");

    let shifted_source = NARRATION.replace(
        "    // sum every item through the iterator\n    items.iter().sum()",
        "    let sum = items.iter().sum();\n    // sum every item through the iterator\n    sum",
    );
    assert_ne!(shifted_source, NARRATION, "the drift variant must differ");
    let shifted = only_outside_channel(&shifted_source, &cfg);
    assert_eq!(shifted.symbol_path, "total", "still the same symbol");
    assert_eq!(shifted.line, base.line + 1, "the comment moved down a line");
    assert_eq!(
        shifted.id, base.id,
        "line drift inside one symbol must not change the id"
    );

    let edited_source = NARRATION.replace("sum every item", "sum each item");
    assert_ne!(edited_source, NARRATION, "the edit variant must differ");
    let edited = only_outside_channel(&edited_source, &cfg);
    assert_eq!(edited.line, base.line, "only the body changed");
    assert_ne!(
        edited.id, base.id,
        "editing one word of the body must change the id"
    );

    let other_symbol_source = NARRATION.replace(
        "pub fn total(items: &[i32]) -> i32 {\n    // sum every item through the iterator\n    items.iter().sum()\n}",
        "pub fn count(items: &[i32]) -> usize {\n    // sum every item through the iterator\n    items.len()\n}",
    );
    assert_ne!(
        other_symbol_source, NARRATION,
        "the other-symbol variant must differ"
    );
    let other = only_outside_channel(&other_symbol_source, &cfg);
    assert_eq!(other.symbol_path, "count", "the comment moved symbols");
    assert_ne!(
        other.id, base.id,
        "the symbol path is part of the id, so moving it changes the id"
    );
}

// spec: Doc summary cap / One-line summary with structured sections passes.
#[test]
fn one_line_summary_with_a_structured_errors_section_passes() {
    let cfg = Config::default();
    let file = analyze("doc_structured.rs", STRUCTURED, &cfg);

    let docs = nodes_of(&file, AttributionClass::Doc);
    assert!(
        docs.iter()
            .any(|node| node.body.trim_start().starts_with("# Errors")),
        "the fixture must carry a `# Errors` section"
    );
    assert!(
        docs.len() >= 4,
        "the fixture must carry a structured section of at least four lines"
    );

    assert!(
        doc_cap::run(&file, &cfg).is_empty(),
        "a one-line summary followed by a blank line and structured sections is capped-clean"
    );
}

// spec: Doc summary cap / Two-line summary fails.
#[test]
fn two_line_doc_lead_exceeds_the_default_cap() {
    let cfg = Config::default();
    let file = analyze("doc_two_line.rs", TWO_LINE, &cfg);

    let docs = nodes_of(&file, AttributionClass::Doc);
    assert_eq!(docs.len(), 2, "the fixture lead is exactly two doc lines");

    let findings = doc_cap::run(&file, &cfg);
    assert_eq!(findings.len(), 1, "one doc block, one finding");
    let finding = &findings[0];
    assert_eq!(finding.check, FindingKind::DocSummaryCapExceeded);
    assert_eq!(finding.path, file.path);
    assert_eq!(finding.line, docs[0].line, "anchored to the first doc line");
    assert_eq!(
        finding.symbol_path, docs[0].symbol_path,
        "the finding inherits the doc node's symbol path"
    );
}

// spec: Doc summary cap / Non-public docstrings are capped equally.
#[test]
fn private_python_docstring_is_capped_like_a_public_one() {
    let cfg = Config::default();
    let file = analyze("nonpublic_docstring.py", NONPUBLIC, &cfg);

    let docs = nodes_of(&file, AttributionClass::Doc);
    assert_eq!(
        docs.len(),
        1,
        "the module docstring is the header channel, not a doc node"
    );
    let node = docs[0];
    assert!(
        node.symbol_path.contains("_private_helper"),
        "the doc node belongs to the private helper, got {:?}",
        node.symbol_path
    );

    let findings = doc_cap::run(&file, &cfg);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].check, FindingKind::DocSummaryCapExceeded);
    assert_eq!(findings[0].line, node.line);
    assert_eq!(findings[0].symbol_path, node.symbol_path);
}

// spec: Doc summary cap / Marker prefix cannot launder a doc comment.
#[test]
fn marker_prefix_cannot_launder_a_doc_comment() {
    let cfg = Config::default();
    let file = analyze("laundering.py", LAUNDERING, &cfg);

    let node = nodes_of(&file, AttributionClass::Doc)
        .into_iter()
        .find(|node| node.body.contains("sad-lcy"))
        .expect("the fixture docstring names a marker id");

    assert_eq!(
        node.class,
        AttributionClass::Doc,
        "a docstring stays documentation even when it begins with the marker prefix"
    );

    let findings = doc_cap::run(&file, &cfg);
    assert_eq!(
        findings.len(),
        1,
        "the marker prefix neither uncaps the summary nor moves it to another channel"
    );
    assert_eq!(findings[0].check, FindingKind::DocSummaryCapExceeded);
    assert_eq!(findings[0].line, node.line);
    assert_eq!(findings[0].symbol_path, node.symbol_path);

    assert!(
        outside_channel::run(&file, &cfg).is_empty(),
        "documentation must never surface as prose"
    );
}

// spec: Doc summary cap / The cap and toggle are configurable.
#[test]
fn doc_summary_cap_and_toggle_control_the_check() {
    let permissive = Config {
        doc_summary_cap: 3,
        ..Config::default()
    };
    let file = analyze("doc_two_line.rs", TWO_LINE, &permissive);
    assert!(
        doc_cap::run(&file, &permissive).is_empty(),
        "a cap of three admits a two-line lead"
    );

    let strict = Config {
        doc_summary_cap: 1,
        ..Config::default()
    };
    let file = analyze("doc_two_line.rs", TWO_LINE, &strict);
    assert_eq!(
        doc_cap::run(&file, &strict).len(),
        1,
        "a cap of one rejects a two-line lead"
    );

    let disabled = Config {
        checks: CheckToggles {
            doc_summary_cap_exceeded: false,
            ..Config::default().checks
        },
        ..Config::default()
    };
    let file = analyze("doc_two_line.rs", TWO_LINE, &disabled);
    assert!(
        doc_cap::run(&file, &disabled).is_empty(),
        "the toggle disables the check regardless of the cap"
    );
}

// spec: Doc summary cap / File-scoped behaviour over doc-class nodes only.
#[test]
fn doc_cap_ignores_prose_and_non_doc_nodes() {
    let cfg = Config::default();

    let todo = analyze("todo.py", TODO, &cfg);
    assert!(
        todo.nodes
            .iter()
            .any(|node| node.class == AttributionClass::Prose),
        "the fixture must carry a prose node"
    );
    assert!(
        doc_cap::run(&todo, &cfg).is_empty(),
        "prose nodes never produce a doc-cap finding"
    );

    let clean = analyze("clean.rs", CLEAN, &cfg);
    assert_eq!(nodes_of(&clean, AttributionClass::Doc).len(), 1);
    assert!(
        doc_cap::run(&clean, &cfg).is_empty(),
        "a single-line doc node stays under the cap"
    );

    let narration = analyze("narration.rs", NARRATION, &cfg);
    assert_eq!(
        outside_channel::run(&narration, &cfg).len(),
        1,
        "the same file carries real prose"
    );
    assert!(
        doc_cap::run(&narration, &cfg).is_empty(),
        "doc_cap only judges doc-class nodes"
    );
}
