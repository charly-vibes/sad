//! Purpose: end-to-end contract tests for `sad check --adopt` and the
//! suppression it buys on later plain runs.
//! Responsibilities: pin the baseline file shape, adoption determinism,
//! suppression of grandfathered ids, and shrink-only re-adoption after a fix.
//! Rationale: adoption is the only sanctioned way to tolerate existing prose,
//! so its file contract is driven through the real binary rather than asserted
//! against internal helpers.

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

fn repo_with_one_error(tag: &str) -> TempRepo {
    let repo = TempRepo::new(tag);
    repo.write("src/lib.rs", NARRATION_SOURCE);
    repo.copy_fixture("clean.rs", "src/clean.rs");
    repo
}

#[test]
fn adopt_writes_the_baseline_contract_and_exits_zero_with_findings() {
    // spec: Baseline adoption / `--adopt` writes sad-baseline.json and exits 0 despite findings.
    let repo = repo_with_one_error("findings");

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
        baseline["schema_version"], 2,
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
    let repo = repo_with_one_error("findings");
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
    let repo = repo_with_one_error("findings");
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
    let repo = repo_with_one_error("findings");

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

const DUPLICATE_PROSE: &str = "\
pub fn total(items: &[i32]) -> i32 {
    // accumulate every item into the running total
    let running_total = items.iter().sum::<i32>();
    // accumulate every item into the running total
    let total = running_total + items.len() as i32;
    total
}
";

/// One prose comment, distinct from [`DUPLICATE_PROSE`]'s body.
const SINGLE_PROSE: &str = "\
pub fn total() -> i32 {
    // add one to the running total
    1
}
";

#[test]
fn duplicate_prose_bodies_get_distinct_ids_and_are_adopted_separately() {
    let repo = TempRepo::new("duplicate-ids");
    repo.write("src/dup.rs", DUPLICATE_PROSE);

    let (code, before) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "prose fails the run: {before}");
    let mut ids = finding_ids(&before);
    assert_eq!(ids.len(), 2, "two nodes are two findings: {before}");
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 2, "identical bodies must not share an id");

    let (code, after) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1);
    let ordered = finding_ids(&after);
    assert_eq!(
        ordered.len(),
        2,
        "the duplicate pair keeps both findings: {after}"
    );
    assert_eq!(
        ordered[1],
        format!("{}#2", ordered[0]),
        "the second occurrence carries the #2 discriminator: {ordered:?}"
    );

    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");
    let adopted = baseline_ids(&repo.read_baseline());
    assert_eq!(
        adopted.len(),
        2,
        "both occurrences occupy baseline entries: {adopted:?}"
    );
    assert!(adopted.contains(&ordered[0]), "{adopted:?}");
    assert!(adopted.contains(&ordered[1]), "{adopted:?}");

    let (code, suppressed) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "both are grandfathered: {suppressed}");
    assert_eq!(suppressed["data"]["stats"]["findings_suppressed"], 2);
}

#[test]
fn a_version_one_baseline_suppresses_then_migrates_once() {
    let repo = TempRepo::new("baseline-migration");
    repo.write("src/lib.rs", SINGLE_PROSE);

    let (_, before) = run_json(repo.path(), &["check", "--all", "--json"]);
    let original_ids = finding_ids(&before);
    assert_eq!(original_ids.len(), 1, "one prose node: {before}");
    repo.write_baseline(&serde_json::json!({
        "schema_version": 1,
        "adopted_at": "2026-01-01T00:00:00Z",
        "finding_ids": original_ids,
    }));

    let (code, plain) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "a v1 baseline still suppresses: {plain}");
    assert_eq!(plain["data"]["stats"]["findings_suppressed"], 1);

    repo.write("src/lib.rs", DUPLICATE_PROSE);
    let (code, debt) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(
        code, 1,
        "the new occurrence reports under the discriminator: {debt}"
    );

    let (code, migrated) = run_adopt(repo.path());
    assert_eq!(
        code, 0,
        "a version-1 baseline may add ids exactly once: {migrated}"
    );
    let baseline = repo.read_baseline();
    assert_eq!(
        baseline["schema_version"], 2,
        "adoption migrates the document: {baseline}"
    );
    let migrated_ids = baseline_ids(&baseline);
    assert_eq!(
        migrated_ids.len(),
        2,
        "both occurrences are adopted by the migration: {migrated_ids:?}"
    );
    assert!(baseline_ids(&repo.read_baseline()).len() > original_ids.len());

    repo.write("src/extra.rs", SINGLE_PROSE);
    let (code, refused) = run_adopt(repo.path());
    assert_eq!(code, 1, "post-migration growth is refused: {refused}");
    assert_names_baseline_growth(&refused);
}

#[test]
fn ids_survive_line_drift_within_a_duplicate_pair() {
    let repo = TempRepo::new("line-drift");
    repo.write("src/dup.rs", DUPLICATE_PROSE);
    let (_, first) = run_json(repo.path(), &["check", "--all", "--json"]);
    let first_ids = finding_ids(&first);
    assert_eq!(first_ids.len(), 2, "{first}");

    repo.write(
        "src/dup.rs",
        &format!("\n\n\n\n\n\n\n\n\n\n{DUPLICATE_PROSE}"),
    );
    let (_, second) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        finding_ids(&second),
        first_ids,
        "inserted lines must not churn finding ids: {second}"
    );
}
