//! Purpose: end-to-end contract tests for the stats block, exemption and
//! exclude accounting, and commit-scoped selection of `sad check`.
//! Responsibilities: pin every stats key and class count, prove excluded and
//! generated files stay out of analysis, and prove the default run honours
//! commit attribution while `--all` does not.
//! Rationale: these outputs decide which files a gate run ever judges, so they
//! are driven through the real binary against disposable fixture repositories.

mod support;

use serde_json::Value;

use support::envelope::*;

#[test]
fn exemptions_and_excludes_keep_files_out_of_analysis() {
    // spec: Exemptions and excludes / generated files and excluded globs are counted but not analyzed.
    let repo = TempRepo::new("exemptions");
    repo.copy_fixture("narration.rs", "vendor/x.rs");
    repo.copy_fixture("generated.rs", "generated.rs");
    repo.write("sad.toml", "exclude = [\"vendor/**\"]\n");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 0,
        "excluded and exempt files cannot fail the run: {envelope}"
    );
    assert_eq!(envelope["ok"], true);
    assert_eq!(
        envelope["data"]["stats"]["files_analyzed"], 0usize,
        "neither the excluded nor the exempt file is analyzed: {envelope}"
    );
    assert!(
        envelope["data"]["stats"]["files_exempt"]
            .as_u64()
            .expect("files_exempt is a count")
            >= 1,
        "the generated file is counted as exempt: {envelope}"
    );
    assert!(
        reported_findings(&envelope).is_empty(),
        "excluded files yield no findings: {envelope}"
    );
    for finding in reported_findings(&envelope) {
        assert!(
            !finding["path"]
                .as_str()
                .unwrap_or("")
                .starts_with("vendor/"),
            "excluded files must never be reported: {finding}"
        );
    }
}

#[test]
fn scope_uses_commit_attribution() {
    // spec: Scope / the default run analyzes only Assisted-by commits while --all analyzes everything.
    let repo = TempRepo::new("scope");
    repo.copy_fixture("plain.rs", "attributed.rs");
    repo.copy_fixture("plain.rs", "legacy.rs");
    git(repo.path(), &["init", "--quiet"]);
    commit_file(
        repo.path(),
        "attributed.rs",
        "add attributed file",
        Some("Assisted-by: Claude"),
    );
    commit_file(repo.path(), "legacy.rs", "add legacy file", None);

    let (code, scoped) = run_json(repo.path(), &["check", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(scoped["ok"], true);
    assert_eq!(
        scoped["data"]["stats"]["files_analyzed"], 1usize,
        "only the attributed file is in scope: {scoped}"
    );

    let (all_code, all) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(all_code, 0);
    assert_eq!(
        all["data"]["stats"]["files_analyzed"], 2usize,
        "--all ignores commit attribution: {all}"
    );
}

#[test]
fn stats_shape_reports_every_class() {
    // spec: Stats / every stat key is present and class counts equal the fixture's node classes.
    let repo = TempRepo::new("stats");
    repo.copy_fixture("all_classes.py", "classes.py");

    let (code, envelope) = run_json(repo.path(), &["check", "--json"]);
    assert_eq!(code, 1, "the one prose node is a finding: {envelope}");

    let stats = &envelope["data"]["stats"];
    for key in [
        "files_discovered",
        "files_excluded",
        "files_selected",
        "files_skipped",
        "files_analyzed",
        "files_exempt",
        "exempt_nodes",
        "comments_header",
        "comments_doc",
        "comments_marker",
        "comments_suppression",
        "comments_prose",
        "suppression_debt",
        "findings",
        "findings_suppressed",
    ] {
        assert!(
            stats.get(key).and_then(Value::as_u64).is_some(),
            "stats.{key} must be a non-negative count: {stats}"
        );
    }

    assert_eq!(stats["files_discovered"], 1usize);
    assert_eq!(stats["files_excluded"], 0usize);
    assert_eq!(stats["files_selected"], 1usize);
    assert_eq!(stats["files_skipped"], 0usize);
    assert_eq!(stats["files_analyzed"], 1usize);
    assert_eq!(stats["files_exempt"], 0usize);
    assert_eq!(stats["exempt_nodes"], 1usize);
    assert_eq!(stats["comments_header"], 1usize);
    assert_eq!(stats["comments_doc"], 1usize);
    assert_eq!(stats["comments_marker"], 1usize);
    assert_eq!(stats["comments_suppression"], 1usize);
    assert_eq!(stats["comments_prose"], 1usize);
    assert_eq!(stats["suppression_debt"], 1usize);
    assert_eq!(stats["findings"], reported_findings(&envelope).len() as u64);
    assert_eq!(
        stats["findings_suppressed"], 0usize,
        "a run without a baseline suppresses nothing: {stats}"
    );
}

#[test]
fn exempt_files_do_not_feed_the_channel_denominator() {
    // spec: Stats / channel counts describe the same non-exempt files as files_analyzed.
    let repo = TempRepo::new("stats-denominator");
    repo.write(
        "generated.rs",
        "// @generated by the fixture generator\n\
         // see sad-lcy: generated marker for the ledger\n\
         fn generated() {}\n",
    );

    let (code, envelope) = run_json(repo.path(), &["check", "--json"]);

    assert_eq!(code, 0, "an exempt file cannot fail the run: {envelope}");
    assert_eq!(envelope["ok"], true);
    let stats = &envelope["data"]["stats"];
    assert_eq!(stats["files_discovered"], 1usize, "{stats}");
    assert_eq!(stats["files_analyzed"], 0usize, "{stats}");
    assert_eq!(stats["files_exempt"], 1usize, "{stats}");
    assert_eq!(stats["exempt_nodes"], 1usize, "{stats}");
    assert_eq!(
        stats["comments_marker"], 0usize,
        "channel counts must skip exempt files, like files_analyzed: {stats}"
    );
    assert_eq!(stats["comments_prose"], 0usize, "{stats}");
    assert!(
        reported_findings(&envelope).is_empty(),
        "an exempt file yields no findings: {envelope}"
    );
}

#[test]
fn untracked_tool_trees_are_never_walked() {
    // spec: Discovery / dependency and cache trees never reach the gate.
    let repo = TempRepo::new("tool-tree");
    repo.copy_fixture("narration.rs", ".venv/lib/x.py");

    let (code, envelope) = run_json(repo.path(), &["check", "--json"]);

    assert_eq!(code, 0, "the venv file must not fail the run: {envelope}");
    assert_eq!(envelope["ok"], true);
    assert!(
        reported_findings(&envelope).is_empty(),
        "the venv file is never analysed: {envelope}"
    );
    let stats = &envelope["data"]["stats"];
    assert_eq!(
        stats["files_discovered"], 0usize,
        "the venv directory never reaches discovery: {stats}"
    );
    assert_eq!(stats["files_analyzed"], 0usize, "{stats}");
    assert_eq!(stats["files_skipped"], 0usize, "{stats}");
}
