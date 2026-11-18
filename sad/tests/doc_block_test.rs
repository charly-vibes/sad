//! Purpose: contract tests for block-documentation extraction and the two
//! document-prose checks (`sad::checks::doc_cap` and
//! `sad::checks::outside_channel`).
//! Responsibilities: pin gutter-stripped block-doc bodies, the summary lead
//! counted in sentences after masking code spans, URLs, abbreviations, and
//! version tokens, one block per documented item across blank lines and
//! attributes, file-level exemptions, and finding-ineligible empty comment
//! bodies.
//! Rationale: `/** ... */` and `///` blocks are documentation only when the
//! delimiter newline and per-line `*` gutter are stripped before the summary
//! cap measures them, a single wrapped sentence must pass however many source
//! lines it spans, and independent blocks must never merge just because their
//! source lines happen to touch.

use std::path::{Path, PathBuf};

use sad::checks::{doc_cap, outside_channel};
use sad::config::{CheckToggles, Config};
use sad::extract::analyze_file;
use sad::model::{
    AnalyzedFile, AttributionClass, CommentNode, Finding, FindingKind, Severity, Smell,
};

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

/// Analyses an in-memory variant of a block-doc fixture's source.
fn analyze_block_variant(name: &str, source: &str) -> AnalyzedFile {
    let path = fixture_path(name);
    let rel_path = format!("tests/fixtures/checks/block_doc/{name}");
    analyze_file(&path, &rel_path, source, &Config::default())
}

fn prose_fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/checks/prose")
        .join(name)
}

/// Analyses a prose fixture exactly as the gate would.
fn analyze_prose(name: &str) -> AnalyzedFile {
    let source = std::fs::read_to_string(prose_fixture_path(name)).unwrap_or_else(|err| {
        panic!("fixture {name} must be readable: {err}");
    });
    analyze_prose_variant(name, &source)
}

/// Analyses an in-memory variant of a prose fixture's source.
fn analyze_prose_variant(name: &str, source: &str) -> AnalyzedFile {
    let path = prose_fixture_path(name);
    let rel_path = format!("tests/fixtures/checks/prose/{name}");
    analyze_file(&path, &rel_path, source, &Config::default())
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
fn rust_block_two_sentence_summary_exceeds_the_cap() {
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
        "a two-sentence block summary must yield exactly one finding: {findings:?}"
    );
    let finding = &findings[0];
    assert_eq!(
        finding.line, 1,
        "the finding anchors to the block's first source line"
    );
    assert_eq!(
        finding.message,
        "doc summary lead spans 2 sentences, exceeding the 1-sentence doc_summary_cap",
        "the message reports sentences, not rendered lines"
    );
    assert_eq!(finding.severity, Severity::Warning);
    assert_eq!(finding.smell, Some(Smell::DocCap));
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
        "two separate one-sentence docs must not merge into a two-sentence lead: {findings:?}"
    );
}

// rule: rustdoc concatenates a `///` block split by a blank source line.
#[test]
fn blank_line_split_doc_block_is_one_documentation_block() {
    let source = std::fs::read_to_string(fixture_path("split_blank.rs"))
        .expect("the blank-split fixture must be readable")
        .replace("line one", "line one.")
        .replace("line two", "Line two.");
    let file = analyze_block_variant("split_blank.rs", &source);
    assert_eq!(
        doc_nodes(&file).len(),
        2,
        "the fixture carries two doc nodes for one item"
    );
    let findings = doc_findings(&file);
    assert_eq!(
        findings.len(),
        1,
        "one item's blank-line-split doc block is a single two-sentence lead: {findings:?}"
    );
    assert_eq!(
        findings[0].message,
        "doc summary lead spans 2 sentences, exceeding the 1-sentence doc_summary_cap",
        "both sentences belong to the same merged block"
    );
}

// rule: an attribute between `///` lines does not split one item's doc block.
#[test]
fn attribute_split_doc_block_is_one_documentation_block() {
    let source = std::fs::read_to_string(fixture_path("split_attribute.rs"))
        .expect("the attribute-split fixture must be readable")
        .replace("line one", "line one.")
        .replace("line two", "Line two.");
    let file = analyze_block_variant("split_attribute.rs", &source);
    assert_eq!(
        doc_nodes(&file).len(),
        2,
        "the fixture carries two doc nodes for one item"
    );
    let findings = doc_findings(&file);
    assert_eq!(
        findings.len(),
        1,
        "one item's attribute-split doc block is a single two-sentence lead: {findings:?}"
    );
    assert_eq!(
        findings[0].message,
        "doc summary lead spans 2 sentences, exceeding the 1-sentence doc_summary_cap",
        "both sentences belong to the same merged block"
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

#[test]
fn wrapped_one_sentence_summary_passes_the_cap() {
    let file = analyze_prose("doc_wrapped.rs");
    let docs = doc_nodes(&file);
    assert!(
        docs.len() >= 4,
        "the fixture's one-sentence summary must wrap over four doc lines, got {}",
        docs.len()
    );
    let findings = doc_findings(&file);
    assert!(
        findings.is_empty(),
        "one wrapped sentence must pass the default one-sentence cap: {findings:?}"
    );
}

#[test]
fn abbreviation_version_and_code_span_periods_do_not_split_sentences() {
    let file = analyze_prose("doc_abbrev.rs");
    assert_eq!(
        doc_nodes(&file).len(),
        3,
        "the fixture carries three documented items"
    );
    let findings = doc_findings(&file);
    assert!(
        findings.is_empty(),
        "abbreviation, version, and inline-code periods must not count as boundaries: {findings:?}"
    );
}

#[test]
fn two_sentences_warn_regardless_of_source_wrapping() {
    let unwrapped = std::fs::read_to_string(prose_fixture_path("doc_two_line.rs"))
        .expect("the two-line fixture must be readable")
        .replace("\n/// Fails", " Fails");
    let file = analyze_prose_variant("doc_two_line.rs", &unwrapped);
    let findings = doc_cap::run(&file, &Config::default());
    assert_eq!(
        findings.len(),
        1,
        "the same two sentences on one source line must warn: {findings:?}"
    );
    assert_eq!(
        findings[0].message,
        "doc summary lead spans 2 sentences, exceeding the 1-sentence doc_summary_cap"
    );
}

#[test]
fn doc_summary_cap_counts_sentences_and_the_toggle_disables_the_check() {
    let file = analyze_prose("doc_two_line.rs");
    assert_eq!(
        doc_nodes(&file).len(),
        2,
        "the fixture lead is exactly two sentences"
    );

    let permissive = Config {
        doc_summary_cap: 3,
        ..Config::default()
    };
    assert!(
        doc_cap::run(&file, &permissive).is_empty(),
        "a cap of three admits two sentences"
    );

    let disabled = Config {
        checks: CheckToggles {
            doc_summary_cap_exceeded: false,
            ..Config::default().checks
        },
        ..Config::default()
    };
    assert!(
        doc_cap::run(&file, &disabled).is_empty(),
        "the toggle disables the check regardless of the cap"
    );
}
