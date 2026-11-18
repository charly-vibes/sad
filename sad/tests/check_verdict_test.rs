//! Purpose: end-to-end contract tests for finding severity and the verdict of
//! `sad check`.
//! Responsibilities: pin the severity and smell shape of every finding, the
//! default versus `--strict` verdict, the blocking counts in JSON and text
//! mode, and suppression directives against disposable fixture repositories.
//! Rationale: severity alone decides whether a run fails, so it is driven
//! through the real binary rather than the pure classification functions.

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

const BANNER_SOURCE: &str = "\
fn separator() {
    // -----
    let total = 0;
}
";

#[test]
fn every_finding_carries_severity_and_smell() {
    let repo = TempRepo::new("severity-shape");
    repo.copy_fixture("missing_header.rs", "src/lib.rs");
    repo.write("sad.toml", "require_header = true\n");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "an error finding blocks: {envelope}");

    let reported = reported_findings(&envelope);
    assert_eq!(
        reported.len(),
        1,
        "only the header finding fires: {envelope}"
    );
    for finding in reported {
        assert!(
            finding.get("severity").is_some(),
            "every finding carries severity: {finding}"
        );
        assert!(
            finding.get("smell").is_some(),
            "every finding carries the smell key: {finding}"
        );
    }
    assert_eq!(reported[0]["severity"], "error");
    assert!(reported[0]["smell"].is_null());
    assert_eq!(
        envelope["data"]["stats"]["findings_blocking"], 1usize,
        "the structural finding is blocking: {envelope}"
    );
}

#[test]
fn warning_findings_pass_by_default_and_fail_strict() {
    let repo = TempRepo::new("warning-only");
    repo.write("src/lib.rs", BANNER_SOURCE);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "warnings must not fail a default run: {envelope}");
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"]["stats"]["findings"], 1usize);
    assert_eq!(envelope["data"]["stats"]["findings_blocking"], 0usize);
    let reported = reported_findings(&envelope);
    assert_eq!(reported.len(), 1, "one banner warning: {envelope}");
    assert_eq!(reported[0]["severity"], "warning");
    assert_eq!(reported[0]["smell"], "banner");

    let text = run_sad(repo.path(), &["check", "--all"]);
    assert_eq!(text.status.code(), Some(0));
    let stdout = String::from_utf8(text.stdout).expect("text stdout is UTF-8");
    assert_eq!(
        stdout.trim_end().lines().count(),
        1,
        "text mode stays one line: {stdout}"
    );
    assert!(
        stdout.contains("pass: 0 blocking, 1 warning in 1 file"),
        "a warning-only pass states the severity split: {stdout}"
    );

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--strict", "--json"]);
    assert_eq!(code, 1, "strict fails on a warning: {envelope}");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"]["stats"]["findings"], 1usize);
    assert_eq!(
        envelope["data"]["stats"]["findings_blocking"], 0usize,
        "strict does not reclassify the warning: {envelope}"
    );
}

#[test]
fn narration_errors_block_the_default_run() {
    let repo = TempRepo::new("narration-error");
    repo.write("src/lib.rs", NARRATION_SOURCE);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "a narration error exits 1: {envelope}");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"]["stats"]["findings"], 1usize);
    assert_eq!(envelope["data"]["stats"]["findings_blocking"], 1usize);
    let reported = reported_findings(&envelope);
    assert_eq!(reported[0]["severity"], "error");
    assert_eq!(reported[0]["smell"], "narration");
}

#[test]
fn a_warning_does_not_mask_a_blocking_error() {
    let repo = TempRepo::new("mixed-severity");
    repo.write("src/error.rs", NARRATION_SOURCE);
    repo.write("src/warning.rs", BANNER_SOURCE);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "the error fails the run: {envelope}");
    assert_eq!(envelope["data"]["stats"]["findings"], 2usize);
    assert_eq!(envelope["data"]["stats"]["findings_blocking"], 1usize);

    let text = run_sad(repo.path(), &["check", "--all"]);
    let stdout = String::from_utf8(text.stdout).expect("text stdout is UTF-8");
    assert!(
        stdout.contains("fail: 1 blocking, 1 warning in 2 files"),
        "text mode splits blocking from warnings: {stdout}"
    );
}

#[test]
fn a_suppression_directive_does_not_silence_an_error() {
    let repo = TempRepo::new("directive-error");
    repo.write(
        "src/lib.rs",
        "\
fn helper() {
    // sad:ignore comment_outside_channel reason: keep the tally clear
    // accumulate every item into the running total
    let total = items.iter().sum();
}
",
    );

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "an error cannot be silenced: {envelope}");
    assert_eq!(envelope["ok"], false);

    let debt = envelope["data"]["suppression_debt"]
        .as_array()
        .expect("suppression debt is an array");
    assert_eq!(debt.len(), 1, "the directive is counted debt: {envelope}");
    assert_eq!(debt[0]["check"], "comment_outside_channel");
    assert_eq!(envelope["data"]["stats"]["suppression_debt"], 1usize);

    let reported = reported_findings(&envelope);
    assert_eq!(reported.len(), 1, "the narration remains: {envelope}");
    assert_eq!(reported[0]["check"], "comment_outside_channel");
    assert_eq!(reported[0]["severity"], "error");
    assert_eq!(envelope["data"]["stats"]["findings_blocking"], 1usize);
}

#[test]
fn structural_findings_carry_error_severity_and_a_null_smell() {
    let repo = TempRepo::new("structural-severity");
    repo.copy_fixture("marker_unresolved.rs", "src/marker_unresolved.rs");
    repo.write("src/debt.py", "# noqa\nx = 1\n# noqa\ny = 2\n");
    repo.write("sad.toml", "suppression_cap = 1\n");
    repo.write_beads_ledger(LEDGER_ID);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "structural findings are errors: {envelope}");

    let reported = reported_findings(&envelope);
    assert_eq!(
        reported.len(),
        2,
        "one marker finding and one cap finding: {envelope}"
    );
    for check in ["unresolvable_marker", "suppression_cap_exceeded"] {
        let finding = reported
            .iter()
            .find(|finding| finding["check"] == check)
            .unwrap_or_else(|| panic!("`{check}` must be reported: {envelope}"));
        assert_eq!(
            finding["severity"], "error",
            "`{check}` is an error: {finding}"
        );
        assert!(
            finding["smell"].is_null(),
            "`{check}` is not derived from prose: {finding}"
        );
    }
    assert_eq!(
        envelope["data"]["stats"]["findings_blocking"], 2usize,
        "both structural findings block: {envelope}"
    );
}
