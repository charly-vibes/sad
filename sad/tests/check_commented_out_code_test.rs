//! Purpose: contract tests for the block-scoped `commented_out_code` check's
//! block emission, grouping, identity, and prose fallback.
//! Responsibilities: drive the check through the same `analyze_file` frontend
//! the gate uses, pin block emission across grammars and comment forms,
//! hold the mixed fixture's observed counts, suppression and doc-channel
//! exclusion, adjacency, ID stability, and the knob-off fallback.
//! Rationale: block identity is a pure function of source text, the pinned
//! grammar, and config; driving real fixtures keeps the tests on the exact
//! nodes and symbol paths the runtime sees, with no hand-built nodes to drift.

mod support;

use std::time::{Duration, Instant};

use sad::checks::commented_out_code::{self, MAX_PREDICATE_BYTES, MAX_PREDICATE_LINES};
use sad::checks::{outside_channel, suppression};
use sad::config::Config;
use sad::model::{AnalyzedFile, AttributionClass, CommentKind, CommentNode, FindingKind, Language};

use support::commented_out_code::*;

/// spec: Commented-out code / Rust block comment is one finding.
#[test]
fn rust_block_comment_is_one_finding() {
    let cfg = Config::default();
    let file = fixture("rust_block_comment.rs");
    let (dead, prose) = split(&file, &cfg);

    assert_eq!(
        dead.len(),
        2,
        "each block comment is judged on its own body"
    );
    assert_eq!(prose.len(), 0);
    assert_eq!(bodies(&dead), vec!["let x = 1;", "fn f() {}"]);
    assert_eq!(dead[1].line, 5, "the gutter block keeps its source anchor");
}

/// spec: Commented-out code / TypeScript and Python equivalents are block findings.
#[test]
fn typescript_and_python_equivalents_are_block_findings() {
    let cfg = Config::default();
    let python = fixture("python_loop.py");
    let (python_dead, python_prose) = split(&python, &cfg);
    assert_eq!(python_dead.len(), 1);
    assert_eq!(python_prose.len(), 0);
    assert_eq!(
        python_dead[0].text, "for i in range(3):\n    print(i)",
        "one block, not one finding per line"
    );
    assert_eq!(python_dead[0].symbol_path, "outer");

    let typescript = fixture("typescript_block.ts");
    let (ts_dead, ts_prose) = split(&typescript, &cfg);
    assert_eq!(ts_dead.len(), 1);
    assert_eq!(ts_prose.len(), 0);
    assert_eq!(ts_dead[0].text, "const x = await load();");
}

/// spec: Commented-out code / Normalization yields identical IDs.
#[test]
fn normalization_yields_identical_ids() {
    let cfg = Config::default();
    let variants = [
        "// fn f() {\n//     g();\n// }\n",
        "// fn f() {\r\n//     g();\r\n// }\r\n",
        "/*\n * fn f() {\n *     g();\n * }\n */\n",
        "/*\r\n * fn f() {\r\n *     g();\r\n * }\r\n */\r\n",
        "//     fn f() {\n//         g();\n//     }\n",
        "/*\n * fn f() {\n *     g();\n * }\r*/\n",
    ];
    let mut ids = Vec::new();
    let mut texts = Vec::new();
    for source in variants {
        let file = source_at("normalized.rs", source);
        let (dead, prose) = split(&file, &cfg);
        assert_eq!(dead.len(), 1, "variant must be one block: {source:?}");
        assert_eq!(prose.len(), 0, "variant must consume its nodes: {source:?}");
        ids.push(dead[0].id.clone());
        texts.push(dead[0].text.clone());
    }
    assert!(
        ids.windows(2).all(|pair| pair[0] == pair[1]),
        "every normalised form must share one id: {ids:?}"
    );
    assert!(
        texts.windows(2).all(|pair| pair[0] == pair[1]),
        "every normalised form must share one body: {texts:?}"
    );
    assert_eq!(texts[0], "fn f() {\n    g();\n}");
}

/// spec: Commented-out code / Mixed file emits block findings and per-node prose.
#[test]
fn mixed_file_emits_block_findings_and_per_node_prose() {
    let cfg = Config::default();
    let file = fixture("mixed.rs");
    let (dead, prose) = split(&file, &cfg);

    assert_eq!(dead.len(), 2, "one finding per dead block: {dead:?}");
    assert_eq!(prose.len(), 3, "three prose findings: {prose:?}");
    assert_kinds(&dead, FindingKind::CommentedOutCode);
    assert_kinds(&prose, FindingKind::CommentOutsideChannel);
    assert_eq!(dead[0].line, 2);
    assert_eq!(dead[1].line, 17);
    assert_eq!(
        file.nodes.len(),
        18,
        "twelve dead lines, one prose line, three dead lines, two prose lines"
    );
}

/// spec: Commented-out code / Suppression id is commented_out_code.
#[test]
fn suppression_id_is_commented_out_code() {
    let cfg = Config::default();
    let file = fixture("suppression.rs");
    let (dead, prose) = split(&file, &cfg);

    assert_eq!(dead.len(), 1, "the directive does not join the block");
    assert_eq!(prose.len(), 0);
    assert_eq!(dead[0].text, "fn dead() {\n    body();\n}");
    assert_eq!(file.nodes.len(), 4);
    assert_eq!(file.nodes[3].class, AttributionClass::Suppression);

    let debt = suppression::entries(&file);
    assert_eq!(debt.len(), 1, "the directive is suppression debt");
    assert_eq!(debt[0].check.as_deref(), Some("commented_out_code"));
}

/// spec: Commented-out code / Doc-channel code examples never enter the class.
#[test]
fn doc_channel_code_examples_never_enter_the_class() {
    let cfg = Config::default();
    let file = fixture("doc_examples.rs");
    let (dead, prose) = split(&file, &cfg);

    assert_eq!(dead.len(), 0);
    assert_eq!(prose.len(), 0);
    assert!(
        file.nodes
            .iter()
            .any(|node| node.class == AttributionClass::Header),
        "the fixture carries a header node"
    );
    assert!(
        file.nodes
            .iter()
            .any(|node| node.class == AttributionClass::Doc),
        "the fixture carries a doc node"
    );
}

/// spec: Commented-out code / Block adjacency joins same-row and splits on separators.
#[test]
fn block_adjacency_joins_same_row_and_splits_on_separators() {
    let cfg = Config::default();
    let file = source_at(
        "same_row.rs",
        "/* fn first() {} */ /* fn second() {} */\n\n/* fn third() {} */\n",
    );
    let (dead, prose) = split(&file, &cfg);

    assert_eq!(file.nodes.len(), 3);
    assert_eq!(
        dead.len(),
        2,
        "same-row nodes join, the separated one does not"
    );
    assert_eq!(prose.len(), 0);
    assert_eq!(dead[0].line, 1);
    assert_eq!(dead[0].text, "fn first() {}\nfn second() {}");
    assert_eq!(dead[1].line, 3);
    assert_eq!(dead[1].text, "fn third() {}");

    let split_by_code = source_at(
        "same_row.rs",
        "/* fn first() {} */\nlet x = 1;\n/* fn second() {} */\n",
    );
    let (code_dead, code_prose) = split(&split_by_code, &cfg);
    assert_eq!(code_dead.len(), 2, "a code line separates the blocks");
    assert_eq!(code_prose.len(), 0);
    assert_eq!(code_dead[0].line, 1);
    assert_eq!(code_dead[1].line, 3);
}

/// spec: Prose outside channels / Matched block replaces per-node prose.
#[test]
fn matched_block_replaces_per_node_prose() {
    let cfg = Config::default();
    let file = fixture("rust_line_block.rs");
    let (dead, _) = commented_out_code::run(&file, &cfg);
    let (_, consumed) = commented_out_code::run(&file, &cfg);

    assert_eq!(dead.len(), 1);
    assert_eq!(consumed.len(), 4, "all four block members are excluded");
    assert_eq!(
        outside_channel::run_excluding(&file, &cfg, &consumed).len(),
        0,
        "no member emits comment_outside_channel"
    );
}

/// spec: Prose outside channels / Disabled check emits per-node prose.
#[test]
fn disabled_check_emits_per_node_prose() {
    let mut cfg = Config::default();
    cfg.checks.commented_out_code = false;
    let file = fixture("mixed.rs");
    let (dead, prose) = split(&file, &cfg);

    assert_eq!(dead.len(), 0, "the disabled check emits no block finding");
    assert_eq!(prose.len(), 18, "every finding-eligible node emits prose");
    assert_eq!(prose.len(), prose_nodes(&file).len());
    assert_kinds(&prose, FindingKind::CommentOutsideChannel);
}

#[test]
fn block_id_is_stable_under_insertion_and_sensitive_to_body_edits() {
    let cfg = Config::default();
    let base = "// fn f() {\n//     g();\n// }\n";
    let shifted = format!("{}{}", "\n".repeat(10), base);
    let edited = "// fn f() {\n//     h();\n// }\n";

    let base_id = source_at("drift.rs", base);
    let shifted_id = source_at("drift.rs", &shifted);
    let edited_id = source_at("drift.rs", edited);

    let (base_dead, _) = split(&base_id, &cfg);
    let (shifted_dead, _) = split(&shifted_id, &cfg);
    let (edited_dead, _) = split(&edited_id, &cfg);

    assert_eq!(base_dead.len(), 1);
    assert_eq!(shifted_dead.len(), 1);
    assert_eq!(edited_dead.len(), 1);
    assert_eq!(
        base_dead[0].id, shifted_dead[0].id,
        "ten inserted lines must not move the block id"
    );
    assert_ne!(
        base_dead[0].id, edited_dead[0].id,
        "a one-character body edit must change the id"
    );
}

#[test]
fn long_prose_runs_stay_bounded_and_keep_per_node_findings() {
    let cfg = Config::default();
    let nodes: Vec<CommentNode> = (0..32_000)
        .map(|index| CommentNode {
            line: index + 2,
            kind: CommentKind::Line,
            class: AttributionClass::Prose,
            symbol_path: "outer".to_string(),
            body: format!("line {index}"),
            text: format!("// line {index}"),
            directive: None,
            next_code_line: None,
        })
        .collect();
    let file = AnalyzedFile {
        path: "long_prose.rs".to_string(),
        language: Language::Rust,
        nodes,
        exempt: false,
        header: false,
    };

    let started = Instant::now();
    let (dead, prose) = split(&file, &cfg);
    let elapsed = started.elapsed();

    assert!(dead.is_empty(), "plain prose never matches the predicate");
    assert_eq!(
        prose.len(),
        32_000,
        "every prose node stays reported under the bound"
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "a 32k-line prose run must not hit the superlinear cliff: {elapsed:?}"
    );
}

#[test]
fn a_code_chunk_past_the_bound_still_yields_one_block() {
    let cfg = Config::default();
    let filler_body_len = 63;
    let lines_per_chunk = MAX_PREDICATE_BYTES / (filler_body_len + 1);
    assert_eq!(
        lines_per_chunk * (filler_body_len + 1),
        MAX_PREDICATE_BYTES,
        "the filler must divide the byte bound exactly"
    );
    let filler = format!("    // {}\n", "a".repeat(filler_body_len));
    let mut source = String::from("fn outer() {\n");
    for _ in 0..lines_per_chunk * 2 {
        source.push_str(&filler);
    }
    source.push_str("    // fn late() {\n");
    source.push_str("    //     let x = 1;\n");
    source.push_str("    //     x\n");
    source.push_str("    // }\n");
    source.push_str("}\n");
    assert!(
        source.len() > MAX_PREDICATE_BYTES,
        "the fixture must exceed the single-parse bound"
    );
    assert!(
        lines_per_chunk * 2 + 4 <= MAX_PREDICATE_LINES,
        "the line bound must not be the active one"
    );
    let file = source_at("long_dead_code.rs", &source);

    let (dead, prose) = split(&file, &cfg);

    assert_eq!(
        dead.len(),
        1,
        "a dead-code item inside one chunk still matches the block: {dead:?}"
    );
    assert_eq!(
        prose.len(),
        0,
        "the matched block consumes every member node: {prose:?}"
    );
    assert!(
        dead[0]
            .text
            .ends_with("fn late() {\n    let x = 1;\n    x\n}"),
        "the finding hashes the whole run body: {:?}",
        dead[0].text
    );
}
