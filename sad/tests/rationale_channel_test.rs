//! Purpose: pin the rationale channel of comment attribution — the closed
//! `SAFETY:` / `INVARIANT:` / `SECURITY:` / `WHY:` tag set, the precedence
//! that keeps header, doc, and marker channels above it, and Rust / Python /
//! TypeScript parity.
//! Responsibilities: assert the class of every node through `analyze_file`,
//! keep tagless `SAFETY:` and `NOTE:` comments in prose, and prove end to end
//! that a rationale comment is counted in `stats.comments_rationale` while
//! emitting no `comment_outside_channel` finding.
//! Rationale: attribution is purely structural, so inline sources driven
//! through the real extractor pin exactly what the gate sees; the end-to-end
//! run keeps the channel counter and the finding suppression from drifting
//! apart.

mod support;

use std::path::Path;

use sad::config::Config;
use sad::extract::analyze_file;
use sad::model::{AnalyzedFile, AttributionClass};

use support::envelope::*;

fn analyze(name: &str, source: &str) -> AnalyzedFile {
    analyze_file(Path::new(name), name, source, &Config::default())
}

#[track_caller]
fn class_of(file: &AnalyzedFile, needle: &str) -> AttributionClass {
    file.nodes
        .iter()
        .find(|node| node.body.contains(needle))
        .unwrap_or_else(|| {
            panic!(
                "{} must carry a comment body containing {needle:?}",
                file.path
            )
        })
        .class
}

const RUST_TAGS: &str = "\
fn tags() {
    // SAFETY: the lock is held
    // INVARIANT: order matters
    // SECURITY: input is validated
    // WHY: callers rely on this order
    let _ = 1;
}
";

const RUST_NON_RATIONALE: &str = "\
fn tags() {
    // SAFETY:
    // NOTE: the lock is held
    let _ = 1;
}
";

const RUST_PRECEDENCE: &str = "\
//! SAFETY: the buffer is pinned

/// WHY: callers rely on this order
pub fn ordered() {
    // see sad-lcy SAFETY: the lock is held
    let _ = 1;
}
";

const PYTHON_TAGS: &str = "\
\"\"\"Purpose: hold the module docstring in the header channel.\"\"\"


def locked() -> None:
    # SAFETY: the lock is held
    pass
";

const TYPESCRIPT_TAGS: &str = "\
/* Purpose: hold the module header in the header channel. */

export function ordered(): number {
    // WHY: callers rely on this order
    return 1;
}
";

#[test]
fn rationale_tags_with_a_body_are_rationale() {
    let file = analyze("x.rs", RUST_TAGS);

    for tag in [
        "SAFETY: the lock is held",
        "INVARIANT: order matters",
        "SECURITY: input is validated",
        "WHY: callers rely on this order",
    ] {
        assert_eq!(
            class_of(&file, tag),
            AttributionClass::Rationale,
            "{tag:?} must attribute to the rationale channel: {:?}",
            file.nodes
        );
    }
}

#[test]
fn tagless_safety_and_note_remain_prose() {
    let file = analyze("x.rs", RUST_NON_RATIONALE);

    assert_eq!(
        class_of(&file, "SAFETY:"),
        AttributionClass::Prose,
        "an empty SAFETY: body is not rationale: {:?}",
        file.nodes
    );
    assert_eq!(
        class_of(&file, "NOTE: the lock is held"),
        AttributionClass::Prose,
        "NOTE: is not in the rationale tag set: {:?}",
        file.nodes
    );
}

#[test]
fn header_doc_and_marker_outrank_rationale() {
    let file = analyze("x.rs", RUST_PRECEDENCE);

    assert_eq!(
        class_of(&file, "the buffer is pinned"),
        AttributionClass::Header,
        "an inner-doc header stays header: {:?}",
        file.nodes
    );
    assert_eq!(
        class_of(&file, "callers rely on this order"),
        AttributionClass::Doc,
        "a doc comment stays doc: {:?}",
        file.nodes
    );
    assert_eq!(
        class_of(&file, "see sad-lcy"),
        AttributionClass::Marker,
        "a leading marker wins over a rationale tag: {:?}",
        file.nodes
    );
}

#[test]
fn python_safety_and_module_docstring_attribute_apart() {
    let file = analyze("x.py", PYTHON_TAGS);

    assert_eq!(
        class_of(&file, "hold the module docstring"),
        AttributionClass::Header,
        "a module docstring is the header channel: {:?}",
        file.nodes
    );
    assert_eq!(
        class_of(&file, "SAFETY: the lock is held"),
        AttributionClass::Rationale,
        "a Python SAFETY: comment is rationale: {:?}",
        file.nodes
    );
}

#[test]
fn typescript_why_and_module_header_attribute_apart() {
    let file = analyze("x.ts", TYPESCRIPT_TAGS);

    assert_eq!(
        class_of(&file, "hold the module header"),
        AttributionClass::Header,
        "a leading module block comment is the header channel: {:?}",
        file.nodes
    );
    assert_eq!(
        class_of(&file, "callers rely on this order"),
        AttributionClass::Rationale,
        "a TypeScript WHY: comment is rationale: {:?}",
        file.nodes
    );
}

#[test]
fn rationale_comments_count_in_stats_without_a_prose_finding() {
    let repo = TempRepo::new("rationale");
    repo.write(
        "rationale.rs",
        "fn locked() {\n    // SAFETY: the lock is held\n    let _ = 1;\n}\n",
    );

    let (code, envelope) = run_json(repo.path(), &["check", "--json"]);

    assert_eq!(code, 0, "a rationale comment is not a finding: {envelope}");
    let stats = &envelope["data"]["stats"];
    assert_eq!(
        stats["comments_rationale"], 1usize,
        "the rationale channel must count its one node: {stats}"
    );
    assert!(
        reported_findings(&envelope)
            .iter()
            .all(|finding| !(finding["check"] == "comment_outside_channel"
                && finding["path"] == "rationale.rs")),
        "a rationale comment must emit no comment_outside_channel finding: {envelope}"
    );
}
