//! Purpose: disposable fixture repositories and envelope accessors shared by
//! the `sad check` contract test binaries.
//! Responsibilities: create per-test temp repos, copy static fixtures, run the
//! real binary, commit files, and read findings out of the JSON envelope.
//! Rationale: the check binaries pin different contracts but must build and
//! drive repositories identically, so that plumbing lives in one place.

use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;
use serde_json::Value;

/// Static inputs copied into each dynamic fixture repository.
pub const FIXTURES: &str = "tests/fixtures/envelope";
/// Fabricated beads id used by fixtures that need a resolvable marker.
pub const LEDGER_ID: &str = "sad-lcy";

/// Unique per-test scratch repository under the system temp dir; removed on
/// drop so no test depends on or leaves workspace state.
pub struct TempRepo(PathBuf);

impl TempRepo {
    pub fn new(tag: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("sad-envelope-{tag}-{}-{id}", std::process::id()));
        // Start empty even if a crashed run left the same name behind.
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp repo is created");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
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

    /// Write raw bytes, so a fixture can be deliberately invalid UTF-8.
    pub fn write_bytes(&self, rel: &str, contents: &[u8]) {
        let destination = self.0.join(rel);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).expect("parent directory is created");
        }
        std::fs::write(&destination, contents)
            .unwrap_or_else(|err| panic!("{} must be writable: {err}", destination.display()));
    }

    pub fn write_beads_ledger(&self, id: &str) {
        self.write(".beads/issues.jsonl", &format!("{{\"id\":\"{id}\"}}\n"));
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

pub fn finding_of_kind<'a>(envelope: &'a Value, check: &str) -> &'a Value {
    reported_findings(envelope)
        .iter()
        .find(|finding| finding["check"].as_str() == Some(check))
        .unwrap_or_else(|| panic!("expected a `{check}` finding, got: {envelope}"))
}

pub fn remediation_description(finding: &Value) -> &str {
    finding["remediation"]["description"]
        .as_str()
        .unwrap_or_else(|| panic!("finding must carry a remediation description: {finding}"))
}

/// `(check id, remediation command)` pairs in a stable order.
pub fn remediation_commands(envelope: &Value) -> Vec<(String, String)> {
    let mut commands: Vec<(String, String)> = reported_findings(envelope)
        .iter()
        .map(|finding| {
            (
                finding["check"]
                    .as_str()
                    .expect("check id is a string")
                    .to_string(),
                finding["remediation"]["command"]
                    .as_str()
                    .expect("remediation command is a string")
                    .to_string(),
            )
        })
        .collect();
    commands.sort();
    commands
}

pub fn git(repo: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=sad envelope tests",
            "-c",
            "user.email=sad-envelope@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .status()
        .expect("git is available");
    assert!(
        status.success(),
        "git {args:?} failed in {}",
        repo.display()
    );
}

/// One git commit a fixture needs: the file, the subject, and the optional
/// attribution trailer the scope rules read.
pub struct Commit<'a> {
    pub file: &'a str,
    pub subject: &'a str,
    pub trailer: Option<&'a str>,
}

impl<'a> Commit<'a> {
    pub fn new(file: &'a str, subject: &'a str) -> Self {
        Self {
            file,
            subject,
            trailer: None,
        }
    }

    pub fn trailer(mut self, trailer: &'a str) -> Self {
        self.trailer = Some(trailer);
        self
    }
}

pub fn commit_file(repo: &Path, commit: Commit<'_>) {
    git(repo, &["add", "--", commit.file]);
    match commit.trailer {
        Some(trailer) => git(
            repo,
            &["commit", "--quiet", "-m", commit.subject, "-m", trailer],
        ),
        None => git(repo, &["commit", "--quiet", "-m", commit.subject]),
    }
}
