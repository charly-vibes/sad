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
    assert_eq!(
        code, 0,
        "the unclassified prose node is a non-blocking warning: {envelope}"
    );
    assert_eq!(envelope["ok"], true);

    let stats = &envelope["data"]["stats"];
    for key in [
        "files_discovered",
        "files_excluded",
        "files_selected",
        "files_skipped",
        "files_attributed",
        "files_without_commit",
        "files_unattributed",
        "files_unsupported",
        "files_analyzed",
        "files_exempt",
        "exempt_nodes",
        "comments_header",
        "comments_doc",
        "comments_marker",
        "comments_rationale",
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
    assert_eq!(stats["files_attributed"], 0usize);
    assert_eq!(
        stats["files_without_commit"], 1usize,
        "an uncommitted fixture has no governing revision: {stats}"
    );
    assert_eq!(stats["files_unattributed"], 0usize);
    assert_eq!(stats["files_unsupported"], 0usize);
    assert_eq!(stats["files_analyzed"], 1usize);
    assert_eq!(stats["files_exempt"], 0usize);
    assert_eq!(stats["exempt_nodes"], 1usize);
    assert_eq!(stats["comments_header"], 1usize);
    assert_eq!(stats["comments_doc"], 1usize);
    assert_eq!(stats["comments_marker"], 1usize);
    assert_eq!(
        stats["comments_rationale"], 0usize,
        "the fixture carries no rationale node: {stats}"
    );
    assert_eq!(stats["comments_suppression"], 1usize);
    assert_eq!(stats["comments_prose"], 1usize);
    assert_eq!(stats["suppression_debt"], 1usize);
    assert_eq!(reported_findings(&envelope).len(), 1);
    assert_eq!(stats["findings"], 1usize);
    assert_eq!(stats["findings"], reported_findings(&envelope).len() as u64);
    assert_eq!(
        stats["findings_blocking"], 0usize,
        "the lone prose finding is a warning, not a blocking error: {stats}"
    );
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

#[test]
fn scope_counts_explain_a_zero_selection_run() {
    let repo = TempRepo::new("scope-zero");
    repo.copy_fixture("plain.rs", "src/lib.rs");
    git(repo.path(), &["init", "--quiet"]);
    commit_file(repo.path(), "src/lib.rs", "add legacy file", None);

    repo.write("src/lib.rs", "// ONLY an uncommitted edit\nfn main() {}\n");

    let (code, envelope) = run_json(repo.path(), &["check", "--json"]);
    assert_eq!(code, 0, "attribution scope is spec-conforming: {envelope}");
    let stats = &envelope["data"]["stats"];
    assert_eq!(stats["files_discovered"], 1usize, "{stats}");
    assert_eq!(
        stats["files_selected"], 0usize,
        "no governing commit carries the trailer: {stats}"
    );
    assert_eq!(stats["files_analyzed"], 0usize, "{stats}");
    assert_eq!(stats["files_attributed"], 0usize, "{stats}");
    assert_eq!(stats["files_without_commit"], 0usize, "{stats}");
    assert_eq!(
        stats["files_unattributed"], 1usize,
        "the skipped legacy file is counted: {stats}"
    );

    let (all_code, all) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(
        all_code, 0,
        "the uncommitted narration is a non-blocking warning: {all}"
    );
    assert_eq!(reported_findings(&all).len(), 1, "{all}");
    assert_eq!(
        all["data"]["stats"]["findings_blocking"], 0usize,
        "a warning-class finding never blocks the run: {all}"
    );
    assert_eq!(
        all["data"]["stats"]["files_selected"], 1usize,
        "--all widens scope: {all}"
    );
}

#[test]
fn javascript_sources_are_gated_through_the_typescript_frontend() {
    let repo = TempRepo::new("javascript");
    repo.write(
        "src/app.js",
        "export function bump(counter) {\n    // increment the counter\n    return counter.increment();\n}\n",
    );
    repo.write(
        "src/mod.mjs",
        "export function reset(state) {\n    // reset the state\n    state.reset();\n}\n",
    );

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 1, "prose in JavaScript fails the run: {envelope}");
    let reported = reported_findings(&envelope);
    let mut paths: Vec<&str> = reported
        .iter()
        .map(|finding| finding["path"].as_str().expect("finding path"))
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        vec!["src/app.js", "src/mod.mjs"],
        "both JavaScript extensions are analysed: {envelope}"
    );
    assert!(
        reported
            .iter()
            .all(|finding| finding["check"] == "comment_outside_channel"),
        "JavaScript prose is an outside-channel finding: {envelope}"
    );
    assert_eq!(
        envelope["data"]["stats"]["files_analyzed"], 2usize,
        "both files reach the analyzer: {envelope}"
    );
}

#[test]
fn unsupported_adjacent_extensions_are_named_in_warnings() {
    let repo = TempRepo::new("unsupported-extensions");
    for name in ["a.tsx", "b.mts", "c.cts", "d.jsx"] {
        repo.write(
            &format!("src/{name}"),
            "export const value = () => {\n    // this narration must never be silent\n    1;\n};\n",
        );
    }

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 0,
        "unsupported extensions yield warnings, not findings: {envelope}"
    );
    assert!(
        reported_findings(&envelope).is_empty(),
        "the gate has no findings for files it cannot parse: {envelope}"
    );
    assert_eq!(
        envelope["data"]["stats"]["files_unsupported"], 4usize,
        "every unsupported file is counted: {envelope}"
    );
    let warnings = envelope["warnings"]
        .as_array()
        .unwrap_or_else(|| panic!("the envelope must carry warnings: {envelope}"));
    let mut paths: Vec<&str> = warnings
        .iter()
        .map(|warning| {
            assert_eq!(
                warning["rule_name"], "unsupported_extension",
                "warnings name the discovery rule: {warning}"
            );
            warning["entity_id"]
                .as_str()
                .unwrap_or_else(|| panic!("warning names the path: {warning}"))
        })
        .collect();
    paths.sort();
    assert_eq!(
        paths,
        vec!["src/a.tsx", "src/b.mts", "src/c.cts", "src/d.jsx"],
        "each skipped extension is named: {envelope}"
    );
    assert!(
        warnings.iter().all(|warning| warning["message"]
            .as_str()
            .is_some_and(|message| message.contains("supported"))),
        "the message states the boundary: {envelope}"
    );
}

#[test]
fn undecodable_sources_count_as_skipped_not_analyzed() {
    let repo = TempRepo::new("stats-skipped");
    repo.write("src/clean.rs", "fn clean() {}\n");
    repo.write_bytes("src/latin1.rs", b"// \xe9\nfn latin() {}\n");
    repo.write_bytes("src/invalid.rs", b"// \xff\xfe\nfn broken() {}\n");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    let stats = &envelope["data"]["stats"];

    assert_eq!(code, 0, "{envelope}");
    assert_eq!(stats["files_discovered"], 3usize, "{stats}");
    assert_eq!(stats["files_selected"], 3usize, "{stats}");
    assert_eq!(stats["files_analyzed"], 1usize, "{stats}");
    assert_eq!(stats["files_skipped"], 2usize, "{stats}");
}
