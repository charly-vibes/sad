//! Purpose: contract tests for suppression debt, the per-file suppression
//! cap, path exclusion globs, and exemption statistics.
//! Responsibilities: pin every directive form's debt entry (path, line,
//! form, check), doc-channel precedence, cap-off/cap-breach behaviour, and
//! the exemption summary that the envelope reports.
//! Rationale: suppression debt is exempt-but-counted, so a missed entry or a
//! silently disabled cap would let an agent route the gate without signal.

use std::path::{Path, PathBuf};

use sad::checks::suppression::{entries, run};
use sad::config::Config;
use sad::exemptions::{is_excluded, summary};
use sad::extract::analyze_file;
use sad::model::{AnalyzedFile, AttributionClass, FindingKind, SuppressionEntry, SuppressionForm};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/checks/suppression")
}

fn rel_path(name: &str) -> String {
    format!("tests/fixtures/checks/suppression/{name}")
}

/// Analyses a suppression fixture on disk exactly as the gate would.
#[track_caller]
fn analyze(name: &str) -> AnalyzedFile {
    let path = fixtures_dir().join(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("fixture {} must be readable: {err}", path.display()));
    analyze_file(&path, &rel_path(name), &source, &Config::default())
}

/// Stable label for a directive form without requiring extra derives.
fn form_label(form: &SuppressionForm) -> &'static str {
    match form {
        SuppressionForm::Noqa => "noqa",
        SuppressionForm::TypeIgnore => "type: ignore",
        SuppressionForm::TsIgnore => "@ts-ignore",
        SuppressionForm::EslintDisable => "eslint-disable",
        SuppressionForm::SadIgnore => "sad:ignore",
    }
}

// spec: Suppression debt is exempt-but-counted / every directive form is counted once with path, line, and form.
#[test]
fn every_directive_form_is_counted_once_in_source_order() {
    let mut all: Vec<SuppressionEntry> = Vec::new();
    for name in ["all_forms.py", "forms.ts", "forms.rs"] {
        let file = analyze(name);
        all.extend(entries(&file));
    }

    assert_eq!(
        all.len(),
        5,
        "five directives in these fixtures must yield exactly five debt entries"
    );

    let actual: Vec<(String, usize, &'static str)> = all
        .iter()
        .map(|entry| (entry.path.clone(), entry.line, form_label(&entry.form)))
        .collect();
    let py = rel_path("all_forms.py");
    let ts = rel_path("forms.ts");
    let rs = rel_path("forms.rs");
    assert_eq!(
        actual,
        vec![
            (py.clone(), 8, "noqa"),
            (py, 9, "type: ignore"),
            (ts, 7, "@ts-ignore"),
            (rs.clone(), 7, "eslint-disable"),
            (rs, 13, "sad:ignore"),
        ],
        "debt entries must follow source order with one entry per directive"
    );

    let sad = all
        .iter()
        .find(|entry| form_label(&entry.form) == "sad:ignore")
        .expect("the sad:ignore directive must be counted");
    assert_eq!(
        sad.check.as_deref(),
        Some("comment_outside_channel"),
        "sad:ignore scopes its debt entry to exactly one check id"
    );
}

// spec: Suppression debt is exempt-but-counted / noqa is counted as suppression, never as prose.
#[test]
fn noqa_node_is_suppression_and_counted() {
    let file = analyze("all_forms.py");
    let noqa = file
        .nodes
        .iter()
        .find(|node| node.line == 8)
        .expect("all_forms.py must carry the noqa comment at line 8");
    assert_eq!(
        noqa.class,
        AttributionClass::Suppression,
        "a noqa comment is a suppression node, not finding-eligible prose"
    );
    assert!(
        noqa.directive.is_some(),
        "the suppression node must carry its parsed directive"
    );

    let counted = entries(&file);
    assert_eq!(counted.len(), 2, "both Python directives must be counted");
    assert!(
        counted
            .iter()
            .any(|entry| entry.line == 8 && form_label(&entry.form) == "noqa"),
        "the noqa debt entry must be present"
    );
}

// spec: Suppression debt is exempt-but-counted / a docstring containing noqa text contributes no debt entry.
#[test]
fn doc_channel_noqa_is_not_suppression_debt() {
    let file = analyze("doc_noqa.py");
    assert!(
        entries(&file).is_empty(),
        "doc precedence wins: a docstring's noqa-looking text is not a directive"
    );

    let doc = file
        .nodes
        .iter()
        .find(|node| node.body.contains("# noqa"))
        .expect("doc_noqa.py must carry the noqa-looking docstring");
    assert_eq!(doc.class, AttributionClass::Doc);
    assert!(
        doc.directive.is_none(),
        "no directive may be parsed out of a doc node"
    );
}

// spec: Suppression debt is exempt-but-counted / the default configuration has no cap.
#[test]
fn default_suppression_cap_is_off() {
    let file = analyze("cap_breach.py");
    assert_eq!(
        entries(&file).len(),
        3,
        "the fixture must carry exactly three directives"
    );

    let cfg = Config::default();
    assert_eq!(cfg.suppression_cap, None, "v0.1 ships with no cap");
    assert!(
        run(&file, &cfg).is_empty(),
        "without a configured cap, suppression debt never becomes a finding"
    );
}

// spec: Suppression debt is exempt-but-counted / a cap breach emits one file-scoped suppression_cap_exceeded.
#[test]
fn cap_breach_emits_one_file_scoped_finding() {
    let file = analyze("cap_breach.py");
    assert_eq!(entries(&file).len(), 3);

    let cfg = Config {
        suppression_cap: Some(2),
        ..Config::default()
    };
    let findings = run(&file, &cfg);
    assert_eq!(
        findings.len(),
        1,
        "one file over its cap must emit exactly one finding"
    );

    let finding = &findings[0];
    assert!(matches!(
        &finding.check,
        FindingKind::SuppressionCapExceeded
    ));
    assert_eq!(
        finding.symbol_path, "-",
        "file-scoped findings use `-` as the symbol path"
    );
    assert_eq!(finding.path, file.path);
    assert!(
        finding.message.contains("cap_breach.py"),
        "the finding message must name the offending file, got {:?}",
        finding.message
    );
    assert!(
        finding.message.contains('2'),
        "the finding message must name the configured cap, got {:?}",
        finding.message
    );
}

// spec: Suppression debt is exempt-but-counted / debt equal to the cap is not a breach.
#[test]
fn cap_equal_to_debt_is_not_a_breach() {
    let file = analyze("cap_breach.py");
    let cfg = Config {
        suppression_cap: Some(3),
        ..Config::default()
    };
    assert!(
        run(&file, &cfg).is_empty(),
        "three directives do not exceed a cap of three"
    );
}

// spec: Suppression debt is exempt-but-counted / disabling the check suppresses cap findings.
#[test]
fn disabled_cap_check_emits_nothing_even_on_breach() {
    let file = analyze("cap_breach.py");
    let mut cfg = Config {
        suppression_cap: Some(2),
        ..Config::default()
    };
    cfg.checks.suppression_cap_exceeded = false;
    assert!(
        run(&file, &cfg).is_empty(),
        "a disabled suppression_cap_exceeded toggle must not emit findings"
    );
}

// spec: Exemptions are observable / exclude globs exempt matching paths at any depth.
#[test]
fn exclude_globs_match_whole_path_segments_at_any_depth() {
    let cfg = Config {
        exclude: vec!["vendor/**".to_string()],
        ..Config::default()
    };
    assert!(is_excluded("vendor/x.rs", &cfg));
    assert!(!is_excluded("src/x.rs", &cfg));
    assert!(is_excluded("vendor/nested/deep/y.py", &cfg));

    let cfg = Config {
        exclude: vec!["**/generated/**".to_string()],
        ..Config::default()
    };
    assert!(
        is_excluded("a/b/generated/c.rs", &cfg),
        "`**/generated/**` must match a generated directory at any depth"
    );
    assert!(
        !is_excluded("a/b/generatedish/c.rs", &cfg),
        "`generated` must match a whole path segment, not a prefix"
    );
}

// spec: Exemptions are observable / envelope reports whole-file exemptions and exempt nodes.
#[test]
fn exemption_summary_counts_whole_files_and_exempt_nodes() {
    let generated = analyze("generated.rs");
    let spdx_only = analyze("spdx_only.rs");
    let plain = analyze("plain.rs");

    assert!(
        generated.exempt,
        "a file whose first block is @generated is exempt"
    );
    assert!(
        spdx_only.exempt,
        "a file carrying only SPDX comments and code is exempt"
    );
    assert!(
        !plain.exempt,
        "SPDX nodes in a file that still carries prose are node-level exemptions only"
    );

    let files = [generated, spdx_only, plain];
    let expected_files = files.iter().filter(|file| file.exempt).count();
    assert_eq!(
        expected_files, 2,
        "the fixtures must contain exactly two whole-file exemptions"
    );

    // Chosen semantics: files_exempt counts files whose `exempt` flag is set;
    // nodes_exempt counts exemption-class nodes across the analyzed files, so
    // it is at least the SPDX nodes in the non-exempt plain file.
    let stats = summary(&files);
    assert_eq!(stats.files_exempt, expected_files);

    let plain_exempt_nodes = files[2]
        .nodes
        .iter()
        .filter(|node| node.class == AttributionClass::Exemption)
        .count();
    assert!(
        plain_exempt_nodes >= 2,
        "plain.rs must carry at least two SPDX exemption nodes, found {plain_exempt_nodes}"
    );
    assert!(
        stats.nodes_exempt >= 2,
        "the SPDX/@generated nodes must surface in the exemption summary"
    );
    assert!(
        stats.nodes_exempt >= plain_exempt_nodes,
        "nodes_exempt must include the plain file's exemption nodes"
    );
}
