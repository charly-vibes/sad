//! Purpose: end-to-end contract tests for the `sad check` envelope — the
//! binary, its JSON body, text rendering, and exit status.
//! Responsibilities: pin findings, a clean run, determinism, CI parity, and
//! text mode against disposable fixture repositories.
//! Rationale: the envelope is the only machine contract; driving the real
//! binary keeps the tests honest about file discovery, config resolution, git
//! scope, and exit codes rather than only the pure check functions.

mod support;

use support::envelope::*;

#[test]
fn findings_fail_the_run_with_a_stable_id_and_remediation() {
    // spec: Check envelope / a prose finding fails the run with a repo-relative anchor and remediation.
    let repo = TempRepo::new("findings");
    repo.copy_fixture("narration.rs", "src/lib.rs");
    repo.write("sad.toml", "require_header = true\n");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 1, "a findings-bearing check must exit 1: {envelope}");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["envelope_kind"], "check");

    let reported = reported_findings(&envelope);
    assert_eq!(
        reported.len(),
        1,
        "only the narration comment is a finding: {envelope}"
    );
    let finding = &reported[0];
    assert_eq!(finding["check"], "comment_outside_channel");
    assert_eq!(finding["path"], "src/lib.rs");
    assert_eq!(finding["symbol_path"], "helper");
    assert_eq!(finding["line"], 6usize);

    let id = finding["id"].as_str().expect("finding id is a string");
    let id_shape =
        regex::Regex::new(r"^src/lib\.rs::.+::[0-9a-f]{16}$").expect("id pattern compiles");
    assert!(
        id_shape.is_match(id),
        "finding id must be path::symbol::hash: {id}"
    );
    assert!(
        id.starts_with("src/lib.rs::helper::"),
        "finding id must name the file and symbol: {id}"
    );

    assert!(
        !finding["message"]
            .as_str()
            .expect("message is a string")
            .trim()
            .is_empty()
    );
    assert!(
        !finding["remediation"]["command"]
            .as_str()
            .expect("remediation command is a string")
            .trim()
            .is_empty()
    );
    assert!(!remediation_description(finding).trim().is_empty());
}

#[test]
fn a_clean_run_passes() {
    // spec: Check envelope / header, doc, and resolvable marker channels yield a passing run.
    let repo = TempRepo::new("clean");
    repo.copy_fixture("clean.rs", "src/lib.rs");
    repo.write_beads_ledger(LEDGER_ID);

    let (code, envelope) = run_json(repo.path(), &["check", "--json"]);

    assert_eq!(code, 0);
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["envelope_kind"], "check");
    assert!(
        reported_findings(&envelope).is_empty(),
        "clean run must report no findings: {envelope}"
    );
    assert_eq!(envelope["data"]["stats"]["findings"], 0usize);
    assert_eq!(envelope["data"]["stats"]["files_analyzed"], 1usize);
}

#[test]
fn repeated_runs_are_deterministic() {
    // spec: Determinism / the same tree yields identical data across runs.
    let repo = TempRepo::new("determinism");
    repo.copy_fixture("narration.rs", "src/lib.rs");
    repo.write("sad.toml", "require_header = true\n");

    let (first_code, first) = run_json(repo.path(), &["check", "--all", "--json"]);
    let (second_code, second) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(first_code, second_code);
    assert_eq!(
        first["data"], second["data"],
        "check data must be deterministic: first={first} second={second}"
    );
    for finding in reported_findings(&first) {
        let path = finding["path"].as_str().expect("path is a string");
        assert!(
            !path.starts_with('/'),
            "finding paths must be repo-relative: {path}"
        );
        assert!(
            !path.contains("sad-envelope-"),
            "finding paths must not leak the temp root: {path}"
        );
    }
}

#[test]
fn text_mode_reports_the_verdict_and_the_first_fix() {
    // spec: Text mode / the human line names the verdict, the counts, and the first fix.
    let findings_repo = TempRepo::new("text-findings");
    findings_repo.copy_fixture("narration.rs", "src/lib.rs");
    findings_repo.write("sad.toml", "require_header = true\n");

    let failing = run_sad(findings_repo.path(), &["check", "--all"]);
    assert_eq!(failing.status.code(), Some(1));
    let failing_stdout = String::from_utf8(failing.stdout).expect("text stdout is UTF-8");
    assert!(
        failing_stdout.contains("fail"),
        "text mode must say fail: {failing_stdout}"
    );
    assert_eq!(
        failing_stdout.trim_end().lines().count(),
        1,
        "text mode stays one line: {failing_stdout}"
    );
    assert!(
        failing_stdout.contains("1 finding in 1 file"),
        "text mode must pluralise correctly: {failing_stdout}"
    );
    assert!(
        failing_stdout.contains("src/lib.rs:6 comment_outside_channel"),
        "text mode must name the first finding: {failing_stdout}"
    );
    assert!(
        failing_stdout.contains("$EDITOR src/lib.rs"),
        "text mode must carry the first remediation command: {failing_stdout}"
    );

    let plural_repo = TempRepo::new("text-plural");
    plural_repo.copy_fixture("narration.rs", "src/a.rs");
    plural_repo.copy_fixture("narration.rs", "src/b.rs");
    plural_repo.write("sad.toml", "require_header = true\n");

    let plural = run_sad(plural_repo.path(), &["check", "--all"]);
    assert_eq!(plural.status.code(), Some(1));
    let plural_stdout = String::from_utf8(plural.stdout).expect("text stdout is UTF-8");
    assert!(
        plural_stdout.contains("2 findings in 2 files"),
        "text mode must pluralise counts: {plural_stdout}"
    );
    assert!(
        plural_stdout.contains("src/a.rs:6 comment_outside_channel"),
        "text mode must name the first finding in path order: {plural_stdout}"
    );

    let clean_repo = TempRepo::new("text-clean");
    clean_repo.copy_fixture("clean.rs", "src/lib.rs");
    clean_repo.write_beads_ledger(LEDGER_ID);

    let passing = run_sad(clean_repo.path(), &["check", "--all"]);
    assert_eq!(passing.status.code(), Some(0));
    let passing_stdout = String::from_utf8(passing.stdout).expect("text stdout is UTF-8");
    assert!(
        passing_stdout.contains("pass"),
        "text mode must say pass: {passing_stdout}"
    );
}

#[test]
fn hooks_and_ci_see_identical_results() {
    // spec: CI parity / repeated invocations over the same tree match in data and exit status.
    let repo = TempRepo::new("parity");
    repo.copy_fixture("narration.rs", "src/lib.rs");
    repo.write("sad.toml", "require_header = true\n");

    let (hook_code, hook) = run_json(repo.path(), &["check", "--all", "--json"]);
    let (ci_code, ci) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        hook_code, ci_code,
        "exit status must match across invocations"
    );
    assert_eq!(hook["ok"], ci["ok"], "ok must match across invocations");
    assert_eq!(
        hook["data"], ci["data"],
        "data must match across invocations: hook={hook} ci={ci}"
    );
}
