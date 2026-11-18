//! Purpose: end-to-end contract tests for `sad check --adopt` and the
//! suppression it buys on later plain runs.
//! Responsibilities: pin the baseline file shape, adoption determinism,
//! suppression of grandfathered ids, and shrink-only re-adoption after a fix.
//! Rationale: adoption is the only sanctioned way to tolerate existing prose,
//! so its file contract is driven through the real binary rather than asserted
//! against internal helpers.

mod support;

use support::baseline::*;

#[test]
fn adopt_writes_the_baseline_contract_and_exits_zero_with_findings() {
    // spec: Baseline adoption / `--adopt` writes sad-baseline.json and exits 0 despite findings.
    let repo = TempRepo::with_findings();

    let (before_code, before) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(before_code, 1, "the pre-adopt run must fail: {before}");
    let mut expected_ids = finding_ids(&before);
    assert_eq!(
        expected_ids.len(),
        1,
        "fixture tree carries exactly one prose finding: {before}"
    );
    expected_ids.sort();
    expected_ids.dedup();

    let (code, envelope) = run_adopt(repo.path());

    assert_eq!(code, 0, "adoption exits 0 even with findings: {envelope}");
    assert_eq!(
        envelope["ok"], true,
        "adoption envelope is a pass: {envelope}"
    );
    assert_eq!(envelope["envelope_kind"], "check");

    let baseline = repo.read_baseline();
    assert_eq!(
        baseline["schema_version"], 1,
        "baseline schema version: {baseline}"
    );

    let adopted_at = baseline["adopted_at"]
        .as_str()
        .unwrap_or_else(|| panic!("adopted_at is a string: {baseline}"));
    let rfc3339_utc =
        regex::Regex::new(r"(?i)^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|\+00:00)$")
            .expect("RFC3339 pattern compiles");
    assert!(
        rfc3339_utc.is_match(adopted_at),
        "adopted_at must be an RFC3339 UTC timestamp: {adopted_at}"
    );

    let ids = baseline_ids(&baseline);
    assert_sorted_and_deduped(&ids);
    assert_eq!(
        ids, expected_ids,
        "baseline must adopt exactly the ids the pre-adopt run reported"
    );
}

#[test]
fn adopted_findings_are_suppressed_by_the_next_plain_run() {
    // spec: Baseline suppression / grandfathered ids stop failing the gate.
    let repo = TempRepo::with_findings();
    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    assert!(
        !baseline_ids(&repo.read_baseline()).is_empty(),
        "the fixture finding must actually be adopted"
    );

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 0, "a grandfathered tree passes: {envelope}");
    assert_eq!(envelope["ok"], true);
    assert_eq!(
        reported_findings(&envelope).len(),
        0,
        "grandfathered ids are suppressed: {envelope}"
    );
    assert_eq!(
        envelope["data"]["stats"]["findings"], 0,
        "suppressed findings are zero new findings: {envelope}"
    );
}

#[test]
fn fixing_prose_passes_without_rewriting_and_re_adoption_shrinks() {
    // spec: Baseline shrink-only / fixing prose passes and re-adoption only shrinks.
    let repo = TempRepo::with_findings();
    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    let original_ids = baseline_ids(&repo.read_baseline());
    assert_eq!(original_ids.len(), 1, "one id is adopted: {original_ids:?}");
    let original_bytes = repo.read_baseline_text();

    // Delete the offending comment by swapping in the clean fixture.
    repo.copy_fixture("clean.rs", "src/lib.rs");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "the fixed tree passes: {envelope}");
    assert_eq!(envelope["ok"], true);
    assert_eq!(
        repo.read_baseline_text(),
        original_bytes,
        "a plain run must not rewrite the baseline"
    );

    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "re-adoption exits 0: {envelope}");
    let readopted = baseline_ids(&repo.read_baseline());
    assert!(
        readopted.len() < original_ids.len(),
        "re-adoption after the fix must shrink finding_ids: {readopted:?} vs {original_ids:?}"
    );
    assert!(
        readopted.is_empty(),
        "with no findings left the baseline is empty: {readopted:?}"
    );
    assert_sorted_and_deduped(&readopted);
}

#[test]
fn adoption_is_deterministic_for_an_unchanged_tree() {
    // spec: Baseline determinism / repeated adoption yields the same finding_ids.
    let repo = TempRepo::with_findings();

    let (code, first) = run_adopt(repo.path());
    assert_eq!(code, 0, "first adoption exits 0: {first}");
    let first_ids = baseline_ids(&repo.read_baseline());
    let first_text = repo.read_baseline_text();

    let (code, second) = run_adopt(repo.path());
    assert_eq!(code, 0, "second adoption exits 0: {second}");
    let second_ids = baseline_ids(&repo.read_baseline());

    assert_eq!(
        first_ids, second_ids,
        "adoption must be deterministic (ignoring adopted_at)"
    );
    assert_eq!(
        repo.read_baseline_text(),
        first_text,
        "an unchanged id set must keep the document byte-stable, adopted_at included"
    );
}
