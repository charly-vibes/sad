//! Purpose: CLI contract tests for `sad check` — the binary is the unit
//! under test.
//! Responsibilities: pin the check envelope on stdout (`ok`, `envelope_kind`,
//! empty `findings`), config auto-discovery from the working directory, the
//! failing envelope for a missing `--config` path, and acceptance of the
//! `--all`/`--adopt` flags.
//! Rationale: machine callers depend on the genesis envelope being the only
//! output format; a bad config path must surface as `ok:false` naming the
//! path, never as clap text or a panic.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;
use predicates::str;

/// Unique per-test scratch directory under the system temp dir; removed on
/// drop. Tests never rely on the workspace CWD.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("sad-check-test-{tag}-{}-{id}", std::process::id()));
        // Start from a truly empty directory even if a crashed earlier run
        // left one behind with the same name.
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp dir is created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sad() -> Command {
    Command::cargo_bin("sad").expect("sad binary builds")
}

#[test]
fn check_json_in_an_empty_directory_emits_an_ok_check_envelope() {
    // No sad.toml and no sources: a clean run is a success envelope of kind
    // `check` with an empty findings array, in compact JSON on stdout.
    let dir = TempDir::new("empty");
    sad()
        .current_dir(dir.path())
        .args(["check", "--json"])
        .assert()
        .success()
        .stdout(str::contains("\"ok\":true"))
        .stdout(str::contains("\"envelope_kind\":\"check\""))
        .stdout(str::contains("\"findings\":[]"));
}

#[test]
fn check_discovers_sad_toml_from_the_working_directory() {
    // A valid sad.toml in the CWD is auto-discovered (no --config flag) and
    // still yields the same clean check envelope.
    let dir = TempDir::new("valid-config");
    std::fs::write(
        dir.path().join("sad.toml"),
        "[checks]\nmissing_header = false\n",
    )
    .expect("sad.toml is written");
    sad()
        .current_dir(dir.path())
        .args(["check", "--json"])
        .assert()
        .success()
        .stdout(str::contains("\"ok\":true"))
        .stdout(str::contains("\"envelope_kind\":\"check\""))
        .stdout(str::contains("\"findings\":[]"));
}

#[test]
fn check_with_missing_config_path_fails_and_names_the_path() {
    // An explicit --config pointing at a nonexistent file is a hard failure:
    // ok:false with a message that names the path the caller passed.
    let dir = TempDir::new("missing-config");
    let missing = dir.path().join("missing-config.toml");
    sad()
        .current_dir(dir.path())
        .args(["check", "--config"])
        .arg(&missing)
        .arg("--json")
        .assert()
        .failure()
        .stdout(str::contains("\"ok\":false"))
        .stdout(str::contains("missing-config.toml"));
}

#[test]
fn check_accepts_all_and_adopt_flags() {
    // `--all` and `--adopt` must be real, accepted flags: an unknown flag
    // would produce the `argument_error` envelope instead of a check
    // envelope. Their semantics are a later ticket's contract.
    let dir = TempDir::new("all-adopt");
    sad()
        .current_dir(dir.path())
        .args(["check", "--all", "--adopt", "--json"])
        .assert()
        .stdout(str::contains("\"envelope_kind\":\"check\""));
}
