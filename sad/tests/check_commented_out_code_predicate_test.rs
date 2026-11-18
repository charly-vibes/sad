//! Purpose: contract tests for the `commented_out_code` predicate's
//! near-miss boundary — the prose that must never enter the class.
//! Responsibilities: pin the parse-error, bare-expression, label,
//! annotation-only, and bodyless-signature near-misses, the pinned
//! documentation false positive, and the prose fallback a predicate miss
//! leaves behind.
//! Rationale: the predicate is the only thing standing between prose and a
//! delete-it finding, so each grammar's near-misses are locked down through
//! the same `analyze_file` frontend and real fixtures the gate uses.

mod support;

use sad::config::Config;
use sad::model::finding_id;

use support::commented_out_code::*;

/// spec: Commented-out code / Parse-error and bare-expression near-misses stay prose.
#[test]
fn parse_error_and_bare_expression_near_misses_stay_prose() {
    let cfg = Config::default();
    let rust = fixture("near_miss.rs");
    let (rust_dead, rust_prose) = split(&rust, &cfg);
    assert_eq!(rust_dead.len(), 0, "no Rust near-miss parses as a unit");
    assert_eq!(rust_prose.len(), 9);
    assert_eq!(prose_nodes(&rust).len(), 9);
    for near_miss in [
        "This function calculates the total.",
        "TODO: retry until stable",
        "https://example.com/spec",
        "Returns Option<String>",
        "let x =",
        "ls -la",
        "5",
        "foo",
    ] {
        assert!(
            rust_prose.iter().any(|finding| finding.text == near_miss),
            "{near_miss:?} must stay prose"
        );
    }

    let python = fixture("near_miss.py");
    let (python_dead, python_prose) = split(&python, &cfg);
    assert_eq!(python_dead.len(), 0);
    assert_eq!(
        bodies(&python_prose),
        vec!["list[str] | None", "TODO: refactor"]
    );
}

/// spec: Commented-out code / Label and annotation-only near-misses stay prose.
#[test]
fn label_and_annotation_only_near_misses_stay_prose() {
    let cfg = Config::default();
    let typescript = fixture("near_miss.ts");
    let (ts_dead, ts_prose) = split(&typescript, &cfg);
    assert_eq!(
        ts_dead.len(),
        0,
        "a labeled statement outranks its call shape"
    );
    assert_eq!(bodies(&ts_prose), vec!["Note: retry();"]);

    let python = fixture("near_miss.py");
    let (python_dead, python_prose) = split(&python, &cfg);
    assert_eq!(python_dead.len(), 0);
    assert!(
        python_prose
            .iter()
            .any(|finding| finding.text == "TODO: refactor"),
        "annotation-only assignment stays prose"
    );
}

/// spec: Commented-out code / Bodyless signature stays prose.
#[test]
fn bodyless_signature_stays_prose() {
    let cfg = Config::default();
    let file = source_at("bodyless.rs", "fn outer() {\n    // fn f();\n}\n");
    let (dead, prose) = split(&file, &cfg);
    assert_eq!(dead.len(), 0);
    assert_eq!(bodies(&prose), vec!["fn f();"]);
}

/// spec: Commented-out code / Documentation call example is a known false positive.
#[test]
fn documentation_call_example_is_a_known_false_positive() {
    let cfg = Config::default();
    let file = fixture("false_positive.rs");
    let (dead, prose) = split(&file, &cfg);

    assert_eq!(dead.len(), 1, "the pinned false positive still fires");
    assert_eq!(dead[0].text, "e.g. g();");
    assert_eq!(prose.len(), 0);
}

/// spec: Prose outside channels / Predicate miss keeps per-node prose.
#[test]
fn predicate_miss_keeps_per_node_prose() {
    let cfg = Config::default();
    let file = fixture("near_miss.rs");
    let (dead, prose) = split(&file, &cfg);

    assert_eq!(dead.len(), 0);
    let eligible = prose_nodes(&file);
    assert_eq!(prose.len(), eligible.len());
    for index in eligible {
        let node = &file.nodes[index];
        let expected = finding_id(&file.path, &node.symbol_path, &node.body);
        assert!(
            prose.iter().any(|finding| finding.id == expected),
            "node {} keeps its unchanged prose id",
            node.line
        );
    }
}
