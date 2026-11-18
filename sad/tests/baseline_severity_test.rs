//! Purpose: end-to-end contract tests for finding severity in `sad check
//! --adopt` and the suppression it buys on later plain runs.
//! Responsibilities: pin adoption of warning-only ids, of mixed warning and
//! error ids, shrink-only re-adoption after a warning is fixed, and the
//! grandfathering of an adopted id whose severity drifts.
//! Rationale: adoption keys on finding ids rather than severity, so the file
//! contract is driven through the real binary rather than internal helpers.

mod support;

use support::baseline::*;

const NARRATION_SOURCE: &str = "\
//! Purpose: hold exactly one prose comment inside a named function.
//! Responsibilities: keep the header channel intact so only narration is flagged.
//! Rationale: an adopted error id must be recorded from this file.

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
fn adoption_records_warning_ids_and_the_next_run_is_clean() {
    let repo = TempRepo::new("warning-adopt");
    repo.write("src/lib.rs", BANNER_SOURCE);

    let (code, before) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "a warning-only run passes by default: {before}");
    assert_eq!(before["data"]["stats"]["findings"], 1usize);
    assert_eq!(before["data"]["stats"]["findings_blocking"], 0usize);
    let warning_ids = finding_ids(&before);
    assert_eq!(warning_ids.len(), 1, "one banner warning: {before}");

    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    assert_eq!(envelope["ok"], true);
    assert_eq!(
        baseline_ids(&repo.read_baseline()),
        warning_ids,
        "warnings are adopted exactly like errors"
    );

    let (code, after) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "the adopted warning is grandfathered: {after}");
    assert_eq!(after["ok"], true);
    assert_eq!(after["data"]["stats"]["findings"], 0usize);
    assert_eq!(after["data"]["stats"]["findings_blocking"], 0usize);
}

#[test]
fn adoption_records_both_warning_and_error_ids() {
    let repo = TempRepo::new("mixed-adopt");
    repo.write("src/error.rs", NARRATION_SOURCE);
    repo.write("src/warning.rs", BANNER_SOURCE);

    let (code, before) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "the error blocks the pre-adopt run: {before}");
    assert_eq!(before["data"]["stats"]["findings"], 2usize);
    assert_eq!(before["data"]["stats"]["findings_blocking"], 1usize);
    let mut expected = finding_ids(&before);
    expected.sort();
    assert_eq!(expected.len(), 2, "both findings have ids: {before}");
    let severities: Vec<&str> = reported_findings(&before)
        .iter()
        .map(|finding| finding["severity"].as_str().expect("severity is a string"))
        .collect();
    assert!(
        severities.contains(&"warning") && severities.contains(&"error"),
        "the tree holds one of each severity: {severities:?}"
    );

    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    assert_eq!(
        baseline_ids(&repo.read_baseline()),
        expected,
        "the baseline records the warning id and the error id"
    );

    let (code, after) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "both adopted ids are grandfathered: {after}");
    assert_eq!(after["data"]["stats"]["findings"], 0usize);
    assert_eq!(after["data"]["stats"]["findings_blocking"], 0usize);
}

#[test]
fn fixing_a_warning_shrinks_the_baseline_on_re_adoption() {
    let repo = TempRepo::new("warning-shrink");
    repo.write("src/lib.rs", BANNER_SOURCE);

    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    let original_ids = baseline_ids(&repo.read_baseline());
    assert_eq!(original_ids.len(), 1, "one warning id is adopted");

    repo.write(
        "src/lib.rs",
        "//! Purpose: carry no finding.\n//! Responsibilities: keep the banner gone.\n\
         //! Rationale: the baseline must shrink.\n\nfn separator() {}\n",
    );

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "the fixed tree passes: {envelope}");
    assert_eq!(envelope["data"]["stats"]["findings"], 0usize);

    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "re-adoption exits 0: {envelope}");
    let readopted = baseline_ids(&repo.read_baseline());
    assert!(
        readopted.len() < original_ids.len(),
        "re-adoption after the fix must shrink the baseline: {readopted:?}"
    );
    assert!(
        readopted.is_empty(),
        "with no findings left the baseline is empty: {readopted:?}"
    );
}

#[test]
fn a_severity_change_to_an_adopted_id_stays_grandfathered() {
    let repo = TempRepo::new("severity-drift");
    repo.write(
        "src/lib.rs",
        "fn run() {\n    // compute totals from the ledger\n    let x = 0;\n}\n",
    );

    let (code, before) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "unclassified prose is a warning: {before}");
    assert_eq!(before["data"]["stats"]["findings"], 1usize);
    assert_eq!(
        reported_findings(&before)[0]["severity"],
        "warning",
        "the comment is not narration yet: {before}"
    );
    let warning_ids = finding_ids(&before);

    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    assert_eq!(
        baseline_ids(&repo.read_baseline()),
        warning_ids,
        "the warning id is adopted"
    );

    repo.write(
        "src/lib.rs",
        "fn run() {\n    // compute totals from the ledger\n    compute_totals(ledger);\n}\n",
    );

    let (code, after) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "the drifted id stays grandfathered: {after}");
    assert_eq!(after["ok"], true);
    assert_eq!(after["data"]["stats"]["findings"], 0usize);
    assert_eq!(after["data"]["stats"]["findings_blocking"], 0usize);
    assert_eq!(
        after["data"]["stats"]["findings_suppressed"], 1usize,
        "the same id is still suppressed: {after}"
    );
    assert_eq!(
        baseline_ids(&repo.read_baseline()),
        warning_ids,
        "a plain run does not rewrite the baseline"
    );
}
