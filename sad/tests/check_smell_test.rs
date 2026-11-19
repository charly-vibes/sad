//! Purpose: contract tests for the shared prose-smell classifier.
//! Responsibilities: pin narration on one-line restatements, sequencer-led
//! calls, three-line runs, and module-level statements; pin apology shapes and
//! the bare-`temporary` fallback; pin banner detection for prose and block
//! bodies; and hold the invariants that classification preserves finding ids,
//! applies one smell to every run member, and offers no bare delete for an
//! informative comment.
//! Rationale: the classifier is a pure post-pass over analysed nodes and
//! emitted findings, so driving it through `analyze_file` and
//! `outside_channel::run` keeps the tests on the exact structural attributions
//! and run boundaries the gate sees at runtime.

use std::collections::BTreeSet;
use std::path::Path;

use sad::checks::{RepoContext, outside_channel, smell};
use sad::config::Config;
use sad::extract::analyze_file;
use sad::model::{AnalyzedFile, Finding, FindingKind, Severity, Smell};
use sad::remediation;

const NARRATION: &str = include_str!("fixtures/checks/prose/narration.rs");
const APOLOGY: &str = include_str!("fixtures/checks/smell/apology.rs");
const BANNER: &str = include_str!("fixtures/checks/smell/banner.rs");
const MULTILINE: &str = include_str!("fixtures/checks/smell/multiline.rs");
const MODULE_LEVEL: &str = include_str!("fixtures/checks/smell/module_level.py");
const TRAILING: &str = include_str!("fixtures/checks/smell/trailing.rs");

const SEQUENCER: &str = "fn main() {\n    // Now: gather the widgets\n    gather_widgets();\n}\n";

fn analyze(rel: &str, source: &str, cfg: &Config) -> AnalyzedFile {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    analyze_file(&path, rel, source, cfg)
}

fn classified(file: &AnalyzedFile, cfg: &Config, context: &RepoContext) -> Vec<Finding> {
    let mut findings = outside_channel::run(file, cfg);
    smell::classify(std::slice::from_ref(file), &mut findings, cfg, context);
    findings
}

#[track_caller]
fn finding<'a>(findings: &'a [Finding], text: &str) -> &'a Finding {
    findings
        .iter()
        .find(|finding| finding.text.trim() == text)
        .unwrap_or_else(|| panic!("no finding with text `{text}`: {findings:?}"))
}

fn marker_context(id: &str) -> RepoContext {
    let mut context = RepoContext::empty();
    context.beads_available = true;
    context.beads_ids.insert(id.to_string());
    context
}

fn block_finding(path: &str, body: &str) -> Finding {
    Finding {
        check: FindingKind::CommentedOutCode,
        id: format!("{path}::{body}"),
        path: path.to_string(),
        line: 1,
        symbol_path: "-".to_string(),
        message: String::new(),
        text: body.to_string(),
        severity: Severity::Warning,
        smell: Some(Smell::CommentedOutCode),
        remediation: remediation::for_smell(Smell::CommentedOutCode, path),
    }
}

#[test]
fn a_two_token_restatement_is_narration() {
    let cfg = Config::default();
    let file = analyze("tests/fixtures/checks/prose/narration.rs", NARRATION, &cfg);
    let findings = classified(&file, &cfg, &RepoContext::empty());

    assert_eq!(
        findings.len(),
        1,
        "one prose node, one finding: {findings:?}"
    );
    assert_eq!(findings[0].smell, Some(Smell::Narration));
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn a_sequencer_naming_the_next_call_is_narration() {
    let cfg = Config::default();
    let file = analyze("tests/fixtures/checks/smell/sequencer.rs", SEQUENCER, &cfg);
    let findings = classified(&file, &cfg, &RepoContext::empty());

    assert_eq!(
        findings.len(),
        1,
        "one prose node, one finding: {findings:?}"
    );
    assert_eq!(findings[0].smell, Some(Smell::Narration));
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn every_member_of_a_three_line_run_carries_narration() {
    let cfg = Config::default();
    let file = analyze("tests/fixtures/checks/smell/multiline.rs", MULTILINE, &cfg);
    let findings = classified(&file, &cfg, &RepoContext::empty());

    assert_eq!(
        findings.len(),
        3,
        "three nodes, three findings: {findings:?}"
    );
    for finding in &findings {
        assert_eq!(finding.smell, Some(Smell::Narration), "{finding:?}");
        assert_eq!(finding.severity, Severity::Error, "{finding:?}");
    }
    let ids: BTreeSet<&str> = findings.iter().map(|finding| finding.id.as_str()).collect();
    assert_eq!(ids.len(), 3, "each member keeps its own id: {ids:?}");
}

#[test]
fn a_module_level_python_statement_is_narrated() {
    let cfg = Config::default();
    let file = analyze(
        "tests/fixtures/checks/smell/module_level.py",
        MODULE_LEVEL,
        &cfg,
    );
    let findings = classified(&file, &cfg, &RepoContext::empty());

    assert_eq!(
        findings.len(),
        1,
        "one prose node, one finding: {findings:?}"
    );
    assert_eq!(findings[0].symbol_path, "-");
    assert_eq!(findings[0].smell, Some(Smell::Narration));
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn bare_temporary_is_unclassified_but_apology_shapes_are_errors() {
    let cfg = Config::default();
    let file = analyze("tests/fixtures/checks/smell/apology.rs", APOLOGY, &cfg);
    let findings = classified(&file, &cfg, &RepoContext::empty());

    assert_eq!(findings.len(), 4, "four prose runs: {findings:?}");
    let bare = finding(&findings, "temporary");
    assert_eq!(bare.smell, Some(Smell::Unclassified));
    assert_eq!(bare.severity, Severity::Warning);

    for text in [
        "temporary fix for the flaky retry",
        "workaround until the parser lands",
        "hack: bypass the cache",
    ] {
        let finding = finding(&findings, text);
        assert_eq!(finding.smell, Some(Smell::Apology), "{text}");
        assert_eq!(finding.severity, Severity::Error, "{text}");
    }
}

#[test]
fn a_resolving_marker_keeps_an_apology_out_of_the_error_set() {
    let cfg = Config::default();
    let source = "fn run() {\n    // workaround see sad-1\n    work();\n}\n";
    let file = analyze(
        "tests/fixtures/checks/smell/marked_apology.rs",
        source,
        &cfg,
    );
    let findings = classified(&file, &cfg, &marker_context("sad-1"));

    assert_eq!(
        findings.len(),
        1,
        "one prose node, one finding: {findings:?}"
    );
    assert_ne!(findings[0].smell, Some(Smell::Apology));
    assert_eq!(findings[0].smell, Some(Smell::Unclassified));
    assert_eq!(findings[0].severity, Severity::Warning);
}

#[test]
fn a_leading_marker_owns_the_comment() {
    let cfg = Config::default();
    let source = "fn run() {\n    // see sad-1 workaround\n    work();\n}\n";
    let file = analyze("tests/fixtures/checks/smell/marker_owned.rs", source, &cfg);
    let findings = classified(&file, &cfg, &marker_context("sad-1"));

    assert!(
        findings.is_empty(),
        "attribution moves a leading cite to the marker channel: {findings:?}"
    );
}

#[test]
fn a_separator_only_prose_body_is_a_banner() {
    let cfg = Config::default();
    let file = analyze("tests/fixtures/checks/smell/banner.rs", BANNER, &cfg);
    let findings = classified(&file, &cfg, &RepoContext::empty());

    assert_eq!(
        findings.len(),
        1,
        "one prose node, one finding: {findings:?}"
    );
    assert_eq!(findings[0].smell, Some(Smell::Banner));
    assert_eq!(findings[0].severity, Severity::Warning);
}

#[test]
fn a_separator_commented_out_code_body_is_a_banner() {
    let cfg = Config::default();
    let analyzed: [AnalyzedFile; 0] = [];
    let mut findings = vec![
        block_finding("src/separator.rs", "----------------"),
        block_finding("src/dead.rs", "let value = 1;"),
    ];
    smell::classify(&analyzed, &mut findings, &cfg, &RepoContext::empty());

    assert_eq!(findings[0].smell, Some(Smell::Banner));
    assert_eq!(findings[0].severity, Severity::Warning);
    assert!(
        findings[0].remediation.description.contains("separator"),
        "{:?}",
        findings[0].remediation.description
    );
    assert_eq!(findings[1].smell, Some(Smell::CommentedOutCode));
    assert_eq!(findings[1].severity, Severity::Warning);
}

#[test]
fn a_comment_with_no_next_statement_is_unclassified() {
    let cfg = Config::default();
    let file = analyze("tests/fixtures/checks/smell/trailing.rs", TRAILING, &cfg);
    assert!(
        file.nodes[0].next_code_line.is_none(),
        "the fixture must end the scope after the comment"
    );
    let findings = classified(&file, &cfg, &RepoContext::empty());

    assert_eq!(
        findings.len(),
        1,
        "one prose node, one finding: {findings:?}"
    );
    assert_eq!(findings[0].smell, Some(Smell::Unclassified));
    assert_eq!(findings[0].severity, Severity::Warning);
}

#[test]
fn classification_preserves_ids_lines_symbols_and_text() {
    let cfg = Config::default();
    let file = analyze("tests/fixtures/checks/smell/multiline.rs", MULTILINE, &cfg);
    let before = outside_channel::run(&file, &cfg);
    let mut after = before.clone();
    smell::classify(
        std::slice::from_ref(&file),
        &mut after,
        &cfg,
        &RepoContext::empty(),
    );

    assert_eq!(before.len(), after.len());
    for (before, after) in before.iter().zip(&after) {
        assert_eq!(before.id, after.id, "classification never rewrites ids");
        assert_eq!(before.line, after.line);
        assert_eq!(before.symbol_path, after.symbol_path);
        assert_eq!(before.text, after.text);
    }
}

#[test]
fn unclassified_guidance_names_channels_and_conditions_deletion() {
    let cfg = Config::default();
    let file = analyze("tests/fixtures/checks/smell/apology.rs", APOLOGY, &cfg);
    let findings = classified(&file, &cfg, &RepoContext::empty());
    let bare = finding(&findings, "temporary");
    let description = bare.remediation.description.to_ascii_lowercase();

    assert!(
        description.contains("channel"),
        "the guidance must name the documented channels: {description}"
    );
    assert!(
        description.contains("if it restates"),
        "deletion must stay conditional on redundancy: {description}"
    );
    assert_ne!(
        description.trim(),
        "delete it.",
        "informative prose never gets a bare delete: {description}"
    );
}

#[test]
fn an_uncited_todo_is_a_todo_smell_and_an_error() {
    let cfg = Config::default();
    let source = "# TODO: make this faster\nvalue = 1\n";
    let file = analyze("tests/fixtures/checks/smell/todo.py", source, &cfg);
    let findings = classified(&file, &cfg, &RepoContext::empty());

    assert_eq!(
        findings.len(),
        1,
        "one prose node, one finding: {findings:?}"
    );
    assert_eq!(findings[0].smell, Some(Smell::Todo));
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn apology_guidance_names_the_marker_channel() {
    let cfg = Config::default();
    let file = analyze("tests/fixtures/checks/smell/apology.rs", APOLOGY, &cfg);
    let findings = classified(&file, &cfg, &RepoContext::empty());
    let apology = finding(&findings, "workaround until the parser lands");
    let description = apology.remediation.description.to_ascii_lowercase();

    assert!(
        description.contains("cite") && description.contains("marker"),
        "the apology fix must name the marker channel: {description}"
    );
    assert!(
        !description.contains("delete"),
        "an apology is never fixed by a bare delete: {description}"
    );
}

/// Smell post-pass: findings on one row from different runs keep their own
/// smell.
#[test]
fn same_row_findings_from_different_runs_keep_their_own_smell() {
    let source = "function a() { /* Now: gather the widgets */ gather_widgets(); } \
                  function b() { /* TODO sad-xss: revisit this later */ return 2; }\n";
    let cfg = Config::default();
    let file = analyze("checks/same_row.ts", source, &cfg);
    let findings = classified(&file, &cfg, &RepoContext::empty());
    assert_eq!(
        findings.len(),
        2,
        "both comments must be findings: {findings:?}"
    );
    let narration = finding(&findings, "Now: gather the widgets");
    let todo = finding(&findings, "TODO sad-xss: revisit this later");
    assert_ne!(
        narration.symbol_path, todo.symbol_path,
        "runs must not join across symbols"
    );
    assert!(
        matches!(narration.smell, Some(Smell::Narration)),
        "narration run must keep its smell, got {:?}",
        narration.smell
    );
    assert!(
        matches!(todo.smell, Some(Smell::Todo)),
        "todo run must keep its smell, got {:?}",
        todo.smell
    );
}
