//! Purpose: disposable fixture repositories and baseline accessors shared by
//! the `sad-baseline.json` contract test binaries.
//! Responsibilities: create per-test temp repos, copy fixtures, run adoption
//! and plain check runs, and read or rewrite the baseline file.
//! Rationale: the adopt and gate binaries pin different halves of one file
//! contract, so they must build and inspect repositories identically.

use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;
use serde_json::Value;

/// Static inputs copied into each disposable repository.
pub const FIXTURES: &str = "tests/fixtures/baseline";
/// The baseline file name fixed by the output contract.
pub const BASELINE_FILE: &str = "sad-baseline.json";

/// Unique per-test scratch repository under the system temp dir; removed on
/// drop so no run leaks state into or reads state from another test.
pub struct TempRepo(PathBuf);

impl TempRepo {
    pub fn new(tag: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("sad-baseline-{tag}-{}-{id}", std::process::id()));
        // Start empty even if a crashed run left the same name behind.
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp repo is created");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn baseline_path(&self) -> PathBuf {
        self.0.join(BASELINE_FILE)
    }

    /// Copy a static fixture into the repo at a repo-relative destination.
    pub fn copy_fixture(&self, fixture: &str, rel: &str) {
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

    pub fn write(&self, rel: &str, contents: &str) {
        let destination = self.0.join(rel);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).expect("parent directory is created");
        }
        std::fs::write(&destination, contents)
            .unwrap_or_else(|err| panic!("{} must be writable: {err}", destination.display()));
    }

    /// The standard two-file tree: one narration finding plus one clean file.
    pub fn with_findings() -> Self {
        let repo = Self::new("findings");
        repo.copy_fixture("narration.rs", "src/lib.rs");
        repo.copy_fixture("clean.rs", "src/clean.rs");
        repo
    }

    pub fn read_baseline_text(&self) -> String {
        let path = self.baseline_path();
        std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("baseline {} must exist: {err}", path.display()))
    }

    pub fn read_baseline(&self) -> Value {
        let text = self.read_baseline_text();
        serde_json::from_str(&text)
            .unwrap_or_else(|err| panic!("baseline must be valid JSON: {err}\n{text}"))
    }

    pub fn write_baseline(&self, value: &Value) {
        let text = serde_json::to_string_pretty(value).expect("baseline serializes");
        self.write(BASELINE_FILE, &text);
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn sad() -> Command {
    Command::cargo_bin("sad").expect("sad binary builds")
}

pub fn run_sad(repo: &Path, args: &[&str]) -> Output {
    sad()
        .current_dir(repo)
        .args(args)
        .output()
        .expect("sad runs")
}

/// Run the binary in `repo` and parse its single JSON envelope from stdout.
pub fn run_json(repo: &Path, args: &[&str]) -> (i32, Value) {
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

pub fn reported_findings(envelope: &Value) -> &[Value] {
    envelope["data"]["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("check envelope must carry a findings array: {envelope}"))
        .as_slice()
}

pub fn finding_ids(envelope: &Value) -> Vec<String> {
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

pub fn baseline_ids(baseline: &Value) -> Vec<String> {
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
pub fn envelope_strings(envelope: &Value) -> Vec<String> {
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

pub fn assert_mentions(envelope: &Value, needle: &str) {
    let strings = envelope_strings(envelope);
    assert!(
        strings.iter().any(|text| text.contains(needle)),
        "envelope must name `{needle}`: {envelope}"
    );
}

/// A gate failure caused by baseline growth, as opposed to a prose finding,
/// must say so in one of its human-readable strings.
pub fn assert_names_baseline_growth(envelope: &Value) {
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

pub fn assert_sorted_and_deduped(ids: &[String]) {
    let mut sorted = ids.to_vec();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        ids, sorted,
        "baseline finding_ids must be sorted and deduped"
    );
}

pub fn run_adopt(repo: &Path) -> (i32, Value) {
    run_json(repo, &["check", "--all", "--adopt", "--json"])
}

/// Run `git` in `repo` and fail the test if it does not succeed.
fn git(repo: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Commit the whole working tree with `message` and no `Assisted-by:`
/// trailer, so a default (attributed-only) run treats every path as legacy.
pub fn commit_all_without_attribution(repo: &Path, message: &str) {
    git(repo, &["init", "-q"]);
    git(repo, &["add", "-A"]);
    git(
        repo,
        &[
            "-c",
            "user.name=sad tests",
            "-c",
            "user.email=sad@example.invalid",
            "commit",
            "-q",
            "--no-gpg-sign",
            "-m",
            message,
        ],
    );
}
