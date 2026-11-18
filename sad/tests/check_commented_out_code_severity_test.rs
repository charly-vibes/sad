//! Purpose: severity contract for the block-scoped `commented_out_code` check —
//! a matched dead-code block is a blocking error that classification preserves.
//! Responsibilities: pin the single-block finding shape, anchor, symbol path,
//! and stable id for the line-comment fixture, and hold the severity, smell, and
//! id across a `smell::classify` pass.
//! Rationale: severity is what makes the check gate a run, so the assertions
//! drive the same `analyze_file` frontend and classification pass the runtime
//! uses instead of constructing findings by hand.

mod support;

use sad::checks::{RepoContext, smell};
use sad::config::Config;
use sad::model::{FindingKind, Severity, Smell, finding_id};

use support::commented_out_code::*;

/// spec: Commented-out code / Rust line-comment block is one finding.
#[test]
fn rust_line_comment_block_is_one_finding() {
    let cfg = Config::default();
    let file = fixture("rust_line_block.rs");
    let (dead, prose) = split(&file, &cfg);

    assert_eq!(dead.len(), 1, "the four-line run is one block: {dead:?}");
    assert_eq!(
        prose.len(),
        0,
        "the matched block replaces every member's prose"
    );
    assert_kinds(&dead, FindingKind::CommentedOutCode);
    let finding = &dead[0];
    let anchor = &file.nodes[0];
    assert_eq!(
        finding.line, anchor.line,
        "anchored at the run's first node"
    );
    assert_eq!(finding.symbol_path, "outer");
    assert_eq!(
        finding.text,
        "fn parse(input: &str) -> usize {\n    let len = input.len();\n    len\n}"
    );
    assert_eq!(
        finding.id,
        finding_id(&file.path, &finding.symbol_path, &finding.text)
    );

    let test_file = fixture("rust_test_block.rs");
    let (test_dead, test_prose) = split(&test_file, &cfg);
    assert_eq!(
        test_dead.len(),
        1,
        "a disabled test is still non-executing code"
    );
    assert_eq!(test_prose.len(), 0);
    assert!(test_dead[0].text.starts_with("#[test]\nfn works()"));
}

/// spec: Findings carry severity / Block findings are errors.
#[test]
fn block_findings_are_errors() {
    let cfg = Config::default();
    let file = fixture("rust_line_block.rs");
    let (mut dead, prose) = split(&file, &cfg);

    assert_eq!(dead.len(), 1);
    assert_eq!(prose.len(), 0);
    assert_eq!(
        dead[0].severity,
        Severity::Error,
        "a matched block is a blocking finding"
    );
    assert_eq!(dead[0].smell, Some(Smell::CommentedOutCode));
    assert_eq!(dead[0].line, file.nodes[0].line);
    let id = dead[0].id.clone();

    smell::classify(
        std::slice::from_ref(&file),
        &mut dead,
        &cfg,
        &RepoContext::empty(),
    );
    assert_eq!(
        dead[0].severity,
        Severity::Error,
        "classification keeps a non-banner block at error"
    );
    assert_eq!(dead[0].smell, Some(Smell::CommentedOutCode));
    assert_eq!(dead[0].id, id, "classification never rewrites the id");
}
