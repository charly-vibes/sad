//! Purpose: end-to-end contract tests for the `sad check` envelope — the
//! binary, its JSON body, text rendering, and exit status.
//! Responsibilities: pin findings, a clean run, determinism, CI parity, and
//! text mode against disposable fixture repositories.
//! Rationale: the envelope is the only machine contract; driving the real
//! binary keeps the tests honest about file discovery, config resolution, git
//! scope, and exit codes rather than only the pure check functions.

mod support;

use support::envelope::*;

const NARRATION_SOURCE: &str = "\
//! Purpose: hold exactly one prose comment inside a named function.
//! Responsibilities: keep the header channel intact so only narration is flagged.
//! Rationale: the envelope test needs a stable symbol-scoped finding.

fn helper() {
    // accumulate every item into the running total
    let total = items.iter().sum();
}
";

#[test]
fn findings_fail_the_run_with_a_stable_id_and_remediation() {
    // spec: Check envelope / a prose finding fails the run with a repo-relative anchor and remediation.
    let repo = TempRepo::new("findings");
    repo.write("src/lib.rs", NARRATION_SOURCE);
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
    findings_repo.write("src/lib.rs", NARRATION_SOURCE);
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
        failing_stdout.contains("fail: 1 blocking, 0 warnings in 1 file"),
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
    plural_repo.write("src/a.rs", NARRATION_SOURCE);
    plural_repo.write("src/b.rs", NARRATION_SOURCE);
    plural_repo.write("sad.toml", "require_header = true\n");

    let plural = run_sad(plural_repo.path(), &["check", "--all"]);
    assert_eq!(plural.status.code(), Some(1));
    let plural_stdout = String::from_utf8(plural.stdout).expect("text stdout is UTF-8");
    assert!(
        plural_stdout.contains("fail: 2 blocking, 0 warnings in 2 files"),
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

#[test]
fn text_mode_names_the_attribution_scope_when_nothing_is_selected() {
    let repo = TempRepo::new("text-scope-zero");
    repo.write("src/lib.rs", "fn main() {}\n");
    git(repo.path(), &["init", "--quiet"]);
    commit_file(repo.path(), "src/lib.rs", "add legacy file", None);
    repo.write(
        "src/lib.rs",
        "// narration added in an uncommitted edit\nfn main() {}\n",
    );

    let scoped = run_sad(repo.path(), &["check"]);
    assert_eq!(scoped.status.code(), Some(0));
    let scoped_stdout = String::from_utf8(scoped.stdout).expect("text stdout is UTF-8");
    assert_eq!(
        scoped_stdout.trim_end().lines().count(),
        1,
        "the hint stays on the text contract's one line: {scoped_stdout}"
    );
    assert!(
        scoped_stdout.contains("0 findings in 0 files"),
        "the counts are unchanged: {scoped_stdout}"
    );
    assert!(
        scoped_stdout.contains("attribution"),
        "text mode must explain why nothing was gated: {scoped_stdout}"
    );
    assert!(
        scoped_stdout.contains("sad check --all"),
        "text mode must name the widening path: {scoped_stdout}"
    );

    let clean = TempRepo::new("text-scope-clean");
    clean.write("src/lib.rs", "//! Purpose: stay clean.\n//! Responsibilities: none.\n//! Rationale: none.\nfn main() {}\n");
    let clean_run = run_sad(clean.path(), &["check"]);
    assert_eq!(clean_run.status.code(), Some(0));
    let clean_stdout = String::from_utf8(clean_run.stdout).expect("text stdout is UTF-8");
    assert!(
        !clean_stdout.contains("attribution"),
        "a genuine clean run carries no scope hint: {clean_stdout}"
    );
}

#[test]
fn skipped_sources_are_named_in_the_envelope_and_the_text() {
    // see sad-o40
    let repo = TempRepo::new("skipped-sources");
    repo.write("src/clean.rs", "fn clean() {}\n");
    repo.write_bytes("src/latin1.rs", b"// \xe9t\xe9\nfn latin() {}\n");
    repo.write_bytes("src/invalid.rs", b"// \xff\xfe broken\nfn broken() {}\n");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "skips alone must not fail the run: {envelope}");
    assert_eq!(envelope["ok"], true);
    assert!(
        reported_findings(&envelope).is_empty(),
        "unreadable content yields no findings: {envelope}"
    );
    assert_eq!(
        envelope["data"]["stats"]["files_skipped"], 2usize,
        "both undecodable files are counted: {envelope}"
    );

    let warnings = envelope["warnings"]
        .as_array()
        .expect("the envelope carries a warnings array");
    let mut skipped: Vec<(&str, &str)> = warnings
        .iter()
        .filter(|warning| warning["rule_name"] == "unreadable_source")
        .map(|warning| {
            (
                warning["entity_id"]
                    .as_str()
                    .expect("a skipped warning names its path"),
                warning["message"]
                    .as_str()
                    .expect("a skipped warning carries a message"),
            )
        })
        .collect();
    skipped.sort();
    assert_eq!(
        skipped.iter().map(|(path, _)| *path).collect::<Vec<&str>>(),
        vec!["src/invalid.rs", "src/latin1.rs"],
        "each skipped file is named exactly once: {envelope}"
    );
    for (path, message) in &skipped {
        assert!(
            message.contains("not valid UTF-8"),
            "`{path}` must carry the stable reason: {message}"
        );
    }

    let text = run_sad(repo.path(), &["check", "--all"]);
    assert_eq!(text.status.code(), Some(0));
    let stdout = String::from_utf8(text.stdout).expect("text stdout is UTF-8");
    assert_eq!(
        stdout.trim_end().lines().count(),
        1,
        "the skipped suffix stays on the one-line contract: {stdout}"
    );
    assert!(
        stdout.contains("pass: 0 findings in 1 file"),
        "the analysed count is unchanged: {stdout}"
    );
    assert!(
        stdout.contains("skipped 2 files"),
        "text mode counts skipped sources: {stdout}"
    );
    assert!(
        stdout.contains("src/invalid.rs (not valid UTF-8)")
            && stdout.contains("src/latin1.rs (not valid UTF-8)"),
        "text mode names every path with its reason: {stdout}"
    );
}

#[test]
fn readable_runs_carry_no_skip_signal() {
    let repo = TempRepo::new("readable-only");
    repo.write("src/clean.rs", "fn clean() {}\n");

    let (_, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(envelope["data"]["stats"]["files_skipped"], 0usize);
    let warnings = envelope["warnings"]
        .as_array()
        .expect("the envelope carries a warnings array");
    assert!(
        warnings
            .iter()
            .all(|warning| warning["rule_name"] != "unreadable_source"),
        "a readable run emits no skip warning: {envelope}"
    );

    let text = run_sad(repo.path(), &["check", "--all"]);
    let stdout = String::from_utf8(text.stdout).expect("text stdout is UTF-8");
    assert!(
        !stdout.contains("skipped"),
        "a readable run keeps the bare one-liner: {stdout}"
    );
}

#[cfg(unix)]
#[test]
fn unreadable_permissions_are_named() {
    use std::os::unix::fs::PermissionsExt;

    let repo = TempRepo::new("unreadable-permissions");
    repo.write("src/blocked.rs", "fn blocked() {}\n");
    let blocked = repo.path().join("src/blocked.rs");
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000))
        .expect("the fixture permissions are set");
    if std::fs::read_to_string(&blocked).is_ok() {
        return;
    }

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(envelope["data"]["stats"]["files_skipped"], 1usize);
    let warning = envelope["warnings"]
        .as_array()
        .expect("the envelope carries a warnings array")
        .iter()
        .find(|warning| warning["rule_name"] == "unreadable_source")
        .unwrap_or_else(|| panic!("an unreadable file is named: {envelope}"));
    assert_eq!(warning["entity_id"], "src/blocked.rs");
    assert!(
        warning["message"]
            .as_str()
            .is_some_and(|message| message.contains("permission denied")),
        "the reason is stable: {warning}"
    );
}
