//! Purpose: end-to-end contract tests for the gate side of the
//! `sad-baseline.json` contract — growth, new findings, corruption, and
//! absence.
//! Responsibilities: prove a superset baseline and an unadopted finding fail
//! the run with a baseline-specific message, and that a corrupt or missing
//! baseline fails closed.
//! Rationale: these are the failure modes that decide whether a baseline can
//! silently authorise prose, so each is driven through the real binary.

mod support;

use support::baseline::*;

#[test]
fn a_superset_baseline_is_rejected_as_growth() {
    // spec: Baseline shrink-only / re-adopting a superset fails the run.
    let repo = TempRepo::with_findings();
    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    let adopted_text = repo.read_baseline_text();

    // A new finding appears after adoption; re-adopting would grow the set.
    repo.copy_fixture("narration.rs", "src/other.rs");

    let (code, envelope) = run_adopt(repo.path());

    assert_eq!(code, 1, "adoption growth is a gate failure: {envelope}");
    assert_eq!(envelope["ok"], false);
    assert_names_baseline_growth(&envelope);
    assert_mentions(&envelope, "sad check --all --adopt");
    assert_eq!(
        repo.read_baseline_text(),
        adopted_text,
        "a refused adoption must not rewrite the baseline"
    );
}

#[test]
fn a_partial_fix_passes_without_rewriting_the_baseline() {
    // spec: Baseline suppression / remaining grandfathered ids do not fail a plain run.
    let repo = TempRepo::new("partial-fix");
    repo.copy_fixture("narration.rs", "src/lib.rs");
    repo.copy_fixture("narration.rs", "src/other.rs");
    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    let adopted = baseline_ids(&repo.read_baseline());
    assert_eq!(adopted.len(), 2, "two findings are adopted: {adopted:?}");
    let adopted_text = repo.read_baseline_text();

    // Fix one of the two adopted findings; the other stays grandfathered.
    repo.copy_fixture("clean.rs", "src/other.rs");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 0,
        "a partially fixed tree still passes on the remaining baseline: {envelope}"
    );
    assert_eq!(envelope["ok"], true);
    assert!(
        reported_findings(&envelope).is_empty(),
        "the remaining grandfathered finding is suppressed: {envelope}"
    );
    assert_eq!(
        envelope["data"]["stats"]["findings_suppressed"], 1,
        "exactly the remaining grandfathered finding is counted: {envelope}"
    );
    assert_eq!(
        repo.read_baseline_text(),
        adopted_text,
        "a plain run must not rewrite the baseline"
    );
}

#[test]
fn out_of_scope_adopted_findings_do_not_fail_a_default_run() {
    // spec: Baseline scope / leftover baseline ids outside the run scope are not staleness.
    let repo = TempRepo::new("out-of-scope");
    repo.copy_fixture("narration.rs", "src/legacy.rs");
    repo.copy_fixture("clean.rs", "src/clean.rs");
    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    assert_eq!(
        baseline_ids(&repo.read_baseline()).len(),
        1,
        "the legacy narration is adopted"
    );
    commit_all_without_attribution(repo.path(), "import legacy tree");

    // No `--all`: the committed file is out of attribution scope, so the
    // baseline carries an id this run never sees.
    let (code, envelope) = run_json(repo.path(), &["check", "--json"]);

    assert_eq!(
        code, 0,
        "an out-of-scope baseline id must not fail a default run: {envelope}"
    );
    assert_eq!(envelope["ok"], true);
    assert_eq!(
        envelope["data"]["stats"]["files_selected"], 0,
        "the legacy file is outside the default scope: {envelope}"
    );
    assert_eq!(reported_findings(&envelope).len(), 0);
}

#[test]
fn a_new_finding_outside_the_baseline_is_still_reported() {
    // spec: Baseline scope / a genuinely new finding is not hidden by the baseline.
    let repo = TempRepo::new("new-finding");
    repo.copy_fixture("narration.rs", "src/lib.rs");
    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    let adopted = baseline_ids(&repo.read_baseline());
    assert_eq!(adopted.len(), 1, "one id is adopted: {adopted:?}");

    // Same prose shape at a new path: a different stable id that is not
    // grandfathered.
    repo.copy_fixture("narration.rs", "src/other.rs");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 1, "an unadopted finding fails the run: {envelope}");
    assert_eq!(envelope["ok"], false);
    let reported = finding_ids(&envelope);
    assert_eq!(
        reported.len(),
        1,
        "only the new id is reported, not the grandfathered one: {envelope}"
    );
    assert!(
        reported[0].starts_with("src/other.rs::"),
        "the new finding is named by its own anchor: {reported:?}"
    );
    assert_eq!(
        baseline_ids(&repo.read_baseline()),
        adopted,
        "a failing plain run must not adopt the new finding"
    );
}

#[test]
fn a_corrupt_baseline_fails_closed_and_names_the_path() {
    // spec: Baseline integrity / corrupt sad-baseline.json fails the run loudly.
    let repo = TempRepo::with_findings();
    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    repo.write(
        BASELINE_FILE,
        "{ this is not the baseline you are looking for",
    );
    repo.write("sad.toml", "");

    let (code, envelope) = run_json(
        repo.path(),
        &["check", "--all", "--json", "--config", "sad.toml"],
    );

    assert_eq!(
        code, 1,
        "a corrupt baseline must not be ignored: {envelope}"
    );
    assert_eq!(envelope["ok"], false);
    assert_mentions(&envelope, BASELINE_FILE);
    assert_mentions(&envelope, "sad check --all --adopt --config sad.toml");
}

#[test]
fn without_a_baseline_the_gate_fails_closed() {
    // spec: Baseline absence / no baseline means findings fail the run.
    let repo = TempRepo::with_findings();

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 1, "findings fail without a baseline: {envelope}");
    assert_eq!(envelope["ok"], false);
    assert_eq!(
        reported_findings(&envelope).len(),
        1,
        "the narration finding is reported: {envelope}"
    );
    assert!(
        !repo.baseline_path().exists(),
        "a plain run must never write a baseline"
    );
}
