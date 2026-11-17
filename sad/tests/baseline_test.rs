//! Purpose: end-to-end contract tests for `sad check --adopt` and the
//! shrink-only `sad-baseline.json` gate.
//! Responsibilities: pin baseline writing, suppression of grandfathered
//! findings, bounded re-adoption, superset rejection, corrupt-baseline
//! failure, fail-closed behaviour without a baseline, and adopt determinism.
//! Rationale: the baseline is the only sanctioned way to tolerate existing
//! prose, so its file shape and gate semantics must be driven through the real
//! binary rather than asserted against internal helpers.

use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;
use serde_json::Value;

/// Static inputs copied into each disposable repository.
const FIXTURES: &str = "tests/fixtures/baseline";
/// The baseline file name fixed by the output contract.
const BASELINE_FILE: &str = "sad-baseline.json";

/// Unique per-test scratch repository under the system temp dir; removed on
/// drop so no run leaks state into or reads state from another test.
struct TempRepo(PathBuf);

impl TempRepo {
    fn new(tag: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("sad-baseline-{tag}-{}-{id}", std::process::id()));
        // Start empty even if a crashed run left the same name behind.
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp repo is created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn baseline_path(&self) -> PathBuf {
        self.0.join(BASELINE_FILE)
    }

    /// Copy a static fixture into the repo at a repo-relative destination.
    fn copy_fixture(&self, fixture: &str, rel: &str) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURES)
            .join(fixture);
        let bytes = std::fs::read(&source)
            .unwrap_or_else(|err| panic!("fixture {} must be readable: {err}", source.display()));
        let destination = self.0.join(rel);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).expect("fixture parent directory is created");
        }
        std::fs::write(&destination, bytes).unwrap_or_else(|err| {
            panic!("fixture {} must be copied: {err}", destination.display())
        });
    }

    fn write(&self, rel: &str, contents: &str) {
        let destination = self.0.join(rel);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).expect("parent directory is created");
        }
        std::fs::write(&destination, contents)
            .unwrap_or_else(|err| panic!("{} must be writable: {err}", destination.display()));
    }

    /// The standard two-file tree: one narration finding plus one clean file.
    fn with_findings() -> Self {
        let repo = Self::new("findings");
        repo.copy_fixture("narration.rs", "src/lib.rs");
        repo.copy_fixture("clean.rs", "src/clean.rs");
        repo
    }

    fn read_baseline_text(&self) -> String {
        let path = self.baseline_path();
        std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("baseline {} must exist: {err}", path.display()))
    }

    fn read_baseline(&self) -> Value {
        let text = self.read_baseline_text();
        serde_json::from_str(&text)
            .unwrap_or_else(|err| panic!("baseline must be valid JSON: {err}\n{text}"))
    }

    fn write_baseline(&self, value: &Value) {
        let text = serde_json::to_string_pretty(value).expect("baseline serializes");
        self.write(BASELINE_FILE, &text);
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sad() -> Command {
    Command::cargo_bin("sad").expect("sad binary builds")
}

fn run_sad(repo: &Path, args: &[&str]) -> Output {
    sad()
        .current_dir(repo)
        .args(args)
        .output()
        .expect("sad runs")
}

/// Run the binary in `repo` and parse its single JSON envelope from stdout.
fn run_json(repo: &Path, args: &[&str]) -> (i32, Value) {
    let output = run_sad(repo, args);
    let stdout = String::from_utf8(output.stdout).expect("envelope stdout is UTF-8");
    let envelope = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!(
            "stdout must be one JSON envelope: {err}\nstdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    let code = output
        .status
        .code()
        .expect("sad exits with an explicit status");
    (code, envelope)
}

fn reported_findings(envelope: &Value) -> &[Value] {
    envelope["data"]["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("check envelope must carry a findings array: {envelope}"))
        .as_slice()
}

fn finding_ids(envelope: &Value) -> Vec<String> {
    reported_findings(envelope)
        .iter()
        .map(|finding| {
            finding["id"]
                .as_str()
                .unwrap_or_else(|| panic!("finding id is a string: {finding}"))
                .to_string()
        })
        .collect()
}

fn baseline_ids(baseline: &Value) -> Vec<String> {
    baseline["finding_ids"]
        .as_array()
        .unwrap_or_else(|| panic!("baseline must carry a finding_ids array: {baseline}"))
        .iter()
        .map(|id| {
            id.as_str()
                .unwrap_or_else(|| panic!("baseline ids are strings: {baseline}"))
                .to_string()
        })
        .collect()
}

/// Every string leaf in an envelope, for failure-message assertions that must
/// not depend on which envelope field a given implementation chooses.
fn envelope_strings(envelope: &Value) -> Vec<String> {
    fn walk(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::String(text) => out.push(text.clone()),
            Value::Array(items) => items.iter().for_each(|item| walk(item, out)),
            Value::Object(map) => map.values().for_each(|item| walk(item, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(envelope, &mut out);
    out
}

fn assert_mentions(envelope: &Value, needle: &str) {
    let strings = envelope_strings(envelope);
    assert!(
        strings.iter().any(|text| text.contains(needle)),
        "envelope must name `{needle}`: {envelope}"
    );
}

/// A gate failure caused by baseline growth, as opposed to a prose finding,
/// must say so in one of its human-readable strings.
fn assert_names_baseline_growth(envelope: &Value) {
    let growth_words = [
        "grow",
        "growth",
        "shrink",
        "larger",
        "superset",
        "new id",
        "new finding",
        "exceed",
        "unreported",
        "not reported",
        "not present",
        "stale",
        "no current",
        "does not match",
        "extra id",
    ];
    let names_growth = envelope_strings(envelope).iter().any(|text| {
        let lowered = text.to_lowercase();
        lowered.contains("baseline") && growth_words.iter().any(|word| lowered.contains(word))
    });
    assert!(
        names_growth,
        "failure must name baseline growth (not a plain prose finding): {envelope}"
    );
}

fn assert_sorted_and_deduped(ids: &[String]) {
    let mut sorted = ids.to_vec();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        ids, sorted,
        "baseline finding_ids must be sorted and deduped"
    );
}

fn run_adopt(repo: &Path) -> (i32, Value) {
    run_json(repo, &["check", "--all", "--adopt", "--json"])
}

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
fn a_superset_baseline_is_rejected_as_growth() {
    // spec: Baseline shrink-only / growing the baseline fails the gate with a baseline message.
    let repo = TempRepo::with_findings();
    let (code, envelope) = run_adopt(repo.path());
    assert_eq!(code, 0, "adoption exits 0: {envelope}");

    let mut baseline = repo.read_baseline();
    let mut ids = baseline_ids(&baseline);
    ids.push("src/lib.rs::total::0000000000000000".to_string());
    ids.sort();
    baseline["finding_ids"] = serde_json::json!(ids);
    repo.write_baseline(&baseline);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 1, "baseline growth is a gate failure: {envelope}");
    assert_eq!(envelope["ok"], false);
    assert_names_baseline_growth(&envelope);
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

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 1,
        "a corrupt baseline must not be ignored: {envelope}"
    );
    assert_eq!(envelope["ok"], false);
    assert_mentions(&envelope, BASELINE_FILE);
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

#[test]
fn adoption_is_deterministic_for_an_unchanged_tree() {
    // spec: Baseline determinism / repeated adoption yields the same finding_ids.
    let repo = TempRepo::with_findings();

    let (code, first) = run_adopt(repo.path());
    assert_eq!(code, 0, "first adoption exits 0: {first}");
    let first_ids = baseline_ids(&repo.read_baseline());

    let (code, second) = run_adopt(repo.path());
    assert_eq!(code, 0, "second adoption exits 0: {second}");
    let second_ids = baseline_ids(&repo.read_baseline());

    assert_eq!(
        first_ids, second_ids,
        "adoption must be deterministic (ignoring adopted_at)"
    );
}
