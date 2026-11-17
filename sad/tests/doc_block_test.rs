//! Purpose: contract tests for block-documentation extraction and the two
//! document-prose checks (`sad::checks::doc_cap` and
//! `sad::checks::outside_channel`).
//! Responsibilities: pin gutter-stripped block-doc bodies, the summary lead
//! measured after normalisation, one block per documented item across blank
//! lines and attributes, file-level exemptions, and finding-ineligible empty
//! comment bodies.
//! Rationale: `/** ... */` and `///` blocks are documentation only when the
//! delimiter newline and per-line `*` gutter are stripped before the summary
//! cap measures them, and independent blocks must never merge just because
//! their source lines happen to touch.

use std::path::{Path, PathBuf};

use sad::checks::{doc_cap, outside_channel};
use sad::config::Config;
use sad::extract::analyze_file;
use sad::model::{AnalyzedFile, AttributionClass, CommentNode, Finding, FindingKind};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/checks/block_doc")
        .join(name)
}

/// Analyses a block-doc fixture exactly as the gate would.
fn analyze(name: &str) -> AnalyzedFile {
    let path = fixture_path(name);
    let rel_path = format!("tests/fixtures/checks/block_doc/{name}");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {} must be readable: {err}", path.display()));
    analyze_file(&path, &rel_path, &source, &Config::default())
}

fn doc_nodes(file: &AnalyzedFile) -> Vec<&CommentNode> {
    file.nodes
        .iter()
        .filter(|node| node.class == AttributionClass::Doc)
        .collect()
}

fn doc_findings(file: &AnalyzedFile) -> Vec<Finding> {
    let findings = doc_cap::run(file, &Config::default());
    assert!(
        findings
            .iter()
            .all(|finding| finding.check == FindingKind::DocSummaryCapExceeded),
        "doc_cap must only emit doc_summary_cap_exceeded findings, got {findings:?}"
    );
    findings
}

// rule: the summary lead ends at the first blank line, so a structured section is uncapped.
#[test]
fn rust_block_structured_doc_passes_the_summary_cap() {
    let file = analyze("rust_block_pass.rs");
    let docs = doc_nodes(&file);
    assert_eq!(
        docs.len(),
        1,
        "the structured block doc must be one doc-class node, got {:?}",
        file.nodes
    );
    let findings = doc_findings(&file);
    assert!(
        findings.is_empty(),
        "a one-line summary followed by a blank line and `# Errors` must pass: {findings:?}"
    );
}

// rule: a two-line block-doc summary exceeds the default one-line cap.
#[test]
fn rust_block_two_line_summary_exceeds_the_cap() {
    let file = analyze("rust_block_two_line.rs");
    assert_eq!(
        doc_nodes(&file).len(),
        1,
        "the two-line block doc must be one doc-class node"
    );
    let findings = doc_findings(&file);
    assert_eq!(
        findings.len(),
        1,
        "a two-line block summary must yield exactly one finding: {findings:?}"
    );
    assert_eq!(
        findings[0].line, 1,
        "the finding anchors to the block's first source line"
    );
}

// rule: a canonical JSDoc with a blank separator and a @throws tag passes.
#[test]
fn typescript_canonical_jsdoc_passes_the_summary_cap() {
    let file = analyze("ts_jsdoc_structured.ts");
    let docs = doc_nodes(&file);
    assert_eq!(
        docs.len(),
        1,
        "the JSDoc must be attributed to the doc channel, got {:?}",
        file.nodes
    );
    let findings = doc_findings(&file);
    assert!(
        findings.is_empty(),
        "a JSDoc summary line plus blank separator and @throws tag must pass: {findings:?}"
    );
}

// rule: independent one-line block docs for adjacent items are never merged.
#[test]
fn adjacent_independent_block_docs_are_not_merged() {
    let file = analyze("adjacent_blocks.rs");
    assert_eq!(
        doc_nodes(&file).len(),
        2,
        "each adjacent helper block is its own doc node"
    );
    let findings = doc_findings(&file);
    assert!(
        findings.is_empty(),
        "two separate one-line docs must not merge into a two-line lead: {findings:?}"
    );
}

// rule: rustdoc concatenates a `///` block split by a blank source line.
#[test]
fn blank_line_split_doc_block_is_one_documentation_block() {
    let file = analyze("split_blank.rs");
    let findings = doc_findings(&file);
    assert_eq!(
        findings.len(),
        1,
        "one item's blank-line-split doc block is a single two-line lead: {findings:?}"
    );
}

// rule: an attribute between `///` lines does not split one item's doc block.
#[test]
fn attribute_split_doc_block_is_one_documentation_block() {
    let file = analyze("split_attribute.rs");
    let findings = doc_findings(&file);
    assert_eq!(
        findings.len(),
        1,
        "one item's attribute-split doc block is a single two-line lead: {findings:?}"
    );
}

// rule: an exempt file (`@generated`) yields no findings from either check.
#[test]
fn exempt_generated_file_produces_no_findings() {
    let file = analyze("exempt_generated.rs");
    assert!(
        file.exempt,
        "the `@generated` marker must exempt the whole file"
    );
    assert!(
        file.nodes
            .iter()
            .any(|node| node.class == AttributionClass::Prose),
        "the fixture must still carry a prose node, or this test is vacuous"
    );
    assert_eq!(
        doc_nodes(&file).len(),
        2,
        "the fixture must still carry a two-line doc block, or this test is vacuous"
    );
    let cfg = Config::default();
    assert!(
        outside_channel::run(&file, &cfg).is_empty(),
        "an exempt file must produce no comment_outside_channel findings"
    );
    assert!(
        doc_cap::run(&file, &cfg).is_empty(),
        "an exempt file must produce no doc_summary_cap_exceeded findings"
    );
}

// rule: an empty comment body (`//`) is not finding-eligible prose.
#[test]
fn empty_separator_comment_is_not_finding_eligible() {
    let file = analyze("empty_separator.rs");
    assert!(
        file.nodes
            .iter()
            .any(|node| node.class == AttributionClass::Prose && node.body.trim().is_empty()),
        "the fixture must carry a bare `//` prose node, or this test is vacuous"
    );
    let findings = outside_channel::run(&file, &Config::default());
    assert_eq!(
        findings.len(),
        1,
        "only the real prose line may produce a finding, got {findings:?}"
    );
    assert_eq!(
        findings[0].text, "note the accumulator below",
        "the surviving finding must carry the real prose body, not the empty separator"
    );
}

// rule: block-doc bodies drop the delimiter newline and per-line `*` gutter.
#[test]
fn rust_block_doc_body_is_gutter_stripped() {
    let file = analyze("rust_block_pass.rs");
    let docs = doc_nodes(&file);
    assert_eq!(docs.len(), 1, "the fixture carries one block doc node");
    let body = &docs[0].body;
    assert!(
        !body.starts_with('\n'),
        "body must not begin with the delimiter newline, got {body:?}"
    );
    assert!(
        body.lines().all(|line| line.trim() != "*"),
        "body must not keep a bare `*` gutter line, got {body:?}"
    );
    assert_eq!(
        body, "Parses config.\n\n# Errors\nReturns an error when the file cannot be read.",
        "body must equal the logical documentation text after gutter stripping"
    );
}
