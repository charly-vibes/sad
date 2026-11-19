//! Purpose: contract tests for comment extraction and channel attribution
//! (`sad::extract` + `sad::model`) across Rust, Python, and TypeScript.
//! Responsibilities: pin one node per attribution class, doc precedence over
//! markers and suppressions, suppression directive capture, string-literal
//! immunity, CRLF/body normalisation, and language detection of each fixture.
//! Rationale: every gate finding is derived from these structural
//! attributions, so each channel must be locked down by realistic fixtures
//! that the pinned tree-sitter grammars parse without error nodes.

mod support;

use std::path::Path;

use sad::config::Config;
use sad::extract::{analyze_file, language_for_path};
use sad::model::{AttributionClass, CommentKind, Language, SuppressionForm};

use support::extract::*;

// spec: Channel attribution / Rust file with one node per class.
#[test]
fn rust_one_node_per_class() {
    let file = analyze("rust", "one_per_class.rs");
    assert!(matches!(file.language, Language::Rust));
    assert_eq!(
        file.path,
        "tests/fixtures/attribution/rust/one_per_class.rs"
    );
    assert!(file.header, "the //! block is the header channel");
    assert!(
        !file.exempt,
        "an isolated SPDX node must not exempt a file that still carries prose"
    );
    assert_nodes(
        &file,
        &[
            (1, CommentKind::Doc, AttributionClass::Header),
            (2, CommentKind::Doc, AttributionClass::Header),
            (3, CommentKind::Doc, AttributionClass::Header),
            (5, CommentKind::Doc, AttributionClass::Doc),
            (7, CommentKind::Line, AttributionClass::Marker),
            (8, CommentKind::Line, AttributionClass::Suppression),
            (12, CommentKind::Line, AttributionClass::Exemption),
            (14, CommentKind::Doc, AttributionClass::Doc),
            (19, CommentKind::Line, AttributionClass::Prose),
        ],
    );
}

// spec: Suppression debt / `sad:ignore` records form, check, and reason.
#[test]
fn rust_sad_ignore_directive_is_captured() {
    let file = analyze("rust", "one_per_class.rs");
    let directive = node_at(&file, 8)
        .directive
        .as_ref()
        .expect("a sad:ignore comment is a suppression directive");
    assert!(matches!(directive.form, SuppressionForm::SadIgnore));
    assert_eq!(
        directive.check.as_deref(),
        Some("comment_outside_channel"),
        "the directive scopes to exactly one check id"
    );
    assert_eq!(directive.reason.as_deref(), Some("reason here"));
}

// spec: Traceability markers resolve / doc comments never route to the marker channel.
#[test]
fn rust_doc_beginning_with_marker_prefix_stays_doc() {
    let file = analyze("rust", "one_per_class.rs");
    let node = node_at(&file, 14);
    assert_eq!(node.class, AttributionClass::Doc);
    assert!(
        node.directive.is_none(),
        "doc precedence wins: no directive parses out of a doc comment"
    );
}

// spec: Channel attribution / bodies strip comment markers and normalise whitespace.
#[test]
fn rust_bodies_strip_markers_and_normalise_whitespace() {
    let file = analyze("rust", "one_per_class.rs");
    assert_eq!(
        node_at(&file, 1).body,
        "Purpose: pin one node per attribution class in Rust.",
        "//! marker and one spacer are stripped from a header line"
    );
    assert_eq!(
        node_at(&file, 5).body,
        "Adds two numbers.",
        "/// marker and one spacer are stripped from a doc line"
    );
    assert_eq!(
        node_at(&file, 19).body,
        "this function calculates the total",
        "// marker and one spacer are stripped from a narration line"
    );
}

// spec: Prose outside channels is a finding / narration-only file is entirely prose.
#[test]
fn rust_narration_only_file_is_all_prose() {
    let file = analyze("rust", "narration_only.rs");
    assert!(!file.header, "no header keywords means no header channel");
    assert!(!file.exempt);
    assert_nodes(
        &file,
        &[
            (1, CommentKind::Line, AttributionClass::Prose),
            (3, CommentKind::Line, AttributionClass::Prose),
        ],
    );
}

// spec: Channel attribution / string literals never produce comment nodes.
#[test]
fn rust_string_literals_do_not_become_comment_nodes() {
    let file = analyze("rust", "string_literals.rs");
    assert_nodes(
        &file,
        &[
            (1, CommentKind::Doc, AttributionClass::Header),
            (2, CommentKind::Doc, AttributionClass::Header),
            (3, CommentKind::Doc, AttributionClass::Header),
            (8, CommentKind::Line, AttributionClass::Prose),
        ],
    );
}

// spec: Channel attribution / CRLF sources classify identically to LF sources.
#[test]
fn rust_crlf_classifies_identically_and_bodies_have_no_carriage_returns() {
    let file = analyze("rust", "crlf.rs");
    assert!(file.header);
    assert_nodes(
        &file,
        &[
            (1, CommentKind::Doc, AttributionClass::Header),
            (2, CommentKind::Doc, AttributionClass::Header),
            (3, CommentKind::Doc, AttributionClass::Header),
            (5, CommentKind::Line, AttributionClass::Marker),
            (7, CommentKind::Doc, AttributionClass::Doc),
            (9, CommentKind::Line, AttributionClass::Prose),
        ],
    );
    for node in &file.nodes {
        assert!(
            !node.body.contains('\r'),
            "body at line {} must be CRLF-normalised, got {:?}",
            node.line,
            node.body
        );
    }
    assert_eq!(node_at(&file, 9).body, "this function calculates the total");
}

// spec: Channel attribution / TypeScript and Python parity with the Rust classes.
#[test]
fn python_one_node_per_class() {
    let file = analyze("python", "one_per_class.py");
    assert!(matches!(file.language, Language::Python));
    assert!(file.header, "the module docstring is the header channel");
    assert!(!file.exempt);
    assert_nodes(
        &file,
        &[
            (1, CommentKind::Doc, AttributionClass::Header),
            (8, CommentKind::Doc, AttributionClass::Doc),
            (10, CommentKind::Line, AttributionClass::Marker),
            (17, CommentKind::Doc, AttributionClass::Doc),
            (18, CommentKind::Line, AttributionClass::Suppression),
            (23, CommentKind::Doc, AttributionClass::Doc),
            (26, CommentKind::Line, AttributionClass::Prose),
        ],
    );
}

// spec: Suppression debt / `# noqa` is counted as a suppression, not flagged as prose.
#[test]
fn python_noqa_is_a_suppression_directive() {
    let file = analyze("python", "one_per_class.py");
    let directive = node_at(&file, 18)
        .directive
        .as_ref()
        .expect("# noqa is a suppression directive");
    assert!(matches!(directive.form, SuppressionForm::Noqa));
    assert!(
        directive.check.is_none(),
        "bare noqa scopes to the whole file, not one check"
    );
}

// spec: Traceability markers resolve / docstrings never route to the marker channel.
#[test]
fn python_docstring_beginning_with_marker_prefix_stays_doc() {
    let file = analyze("python", "one_per_class.py");
    let node = node_at(&file, 17);
    assert_eq!(node.class, AttributionClass::Doc);
    assert!(
        node.directive.is_none(),
        "a docstring is documentation even when its text names a marker"
    );
}

// spec: Suppression debt / suppression text inside a docstring stays documentation.
#[test]
fn python_noqa_inside_docstring_stays_doc() {
    let file = analyze("python", "one_per_class.py");
    let node = node_at(&file, 23);
    assert_eq!(node.class, AttributionClass::Doc);
    assert!(
        node.directive.is_none(),
        "doc precedence wins: directives only classify non-doc nodes"
    );
}

// spec: Channel attribution / Python string literals never produce comment nodes.
#[test]
fn python_string_literals_do_not_become_comment_nodes() {
    let file = analyze("python", "string_literals.py");
    assert_nodes(
        &file,
        &[
            (1, CommentKind::Doc, AttributionClass::Header),
            (8, CommentKind::Line, AttributionClass::Prose),
        ],
    );
}

// spec: Channel attribution / TypeScript and Python parity with the Rust classes.
#[test]
fn typescript_one_node_per_class() {
    let file = analyze("typescript", "one_per_class.ts");
    assert!(matches!(file.language, Language::TypeScript));
    assert!(
        file.header,
        "the leading block comment is the header channel"
    );
    assert!(!file.exempt);
    assert_nodes(
        &file,
        &[
            (1, CommentKind::Block, AttributionClass::Header),
            (5, CommentKind::Doc, AttributionClass::Doc),
            (7, CommentKind::Line, AttributionClass::Marker),
            (8, CommentKind::Line, AttributionClass::Suppression),
            (13, CommentKind::Line, AttributionClass::Prose),
        ],
    );
}

// spec: Suppression debt / `@ts-ignore` is recognized in any comment node kind.
#[test]
fn typescript_ts_ignore_is_a_suppression_directive() {
    let file = analyze("typescript", "one_per_class.ts");
    let directive = node_at(&file, 8)
        .directive
        .as_ref()
        .expect("@ts-ignore is a suppression directive");
    assert!(matches!(directive.form, SuppressionForm::TsIgnore));
    assert!(
        directive.check.is_none(),
        "ts-ignore scopes to the lines below, not one check id"
    );
}

// spec: Channel attribution / TS template literals never produce comment nodes.
#[test]
fn typescript_string_and_template_literals_do_not_become_comment_nodes() {
    let file = analyze("typescript", "string_literals.ts");
    assert_nodes(
        &file,
        &[
            (1, CommentKind::Block, AttributionClass::Header),
            (8, CommentKind::Line, AttributionClass::Prose),
        ],
    );
}

#[test]
fn pyi_stubs_are_analyzed_as_python() {
    const STUB: &str = "\"\"\"Purpose: stub-only module docstring.\"\"\"\n\n\nclass Client:\n    \"\"\"Purpose: stub-only class docstring.\"\"\"\n\n    def send(self, payload: str) -> None: ...\n";
    let path = Path::new("tests/fixtures/attribution/python/stub.pyi");

    assert_eq!(
        language_for_path(path),
        Some(Language::Python),
        ".pyi stubs route to the Python frontend"
    );

    let file = analyze_file(
        path,
        "tests/fixtures/attribution/python/stub.pyi",
        STUB,
        &Config::default(),
    );
    assert_eq!(file.language, Language::Python);
    assert!(file.header, "the module docstring is the header channel");
    assert_nodes(
        &file,
        &[
            (1, CommentKind::Doc, AttributionClass::Header),
            (5, CommentKind::Doc, AttributionClass::Doc),
        ],
    );
}

// spec: Channel attribution / Lone-CR sources classify identically to LF sources.
#[test]
fn rust_lone_cr_classifies_identically_and_lines_are_counted() {
    let file = analyze("rust", "cr.rs");
    assert!(file.header);
    assert_nodes(
        &file,
        &[
            (1, CommentKind::Doc, AttributionClass::Header),
            (2, CommentKind::Doc, AttributionClass::Header),
            (4, CommentKind::Line, AttributionClass::Marker),
            (6, CommentKind::Line, AttributionClass::Prose),
        ],
    );
    for node in &file.nodes {
        assert!(
            !node.body.contains('\r'),
            "body at line {} must be CR-normalised, got {:?}",
            node.line,
            node.body
        );
    }
    assert_eq!(node_at(&file, 6).body, "this function calculates the total");
}
