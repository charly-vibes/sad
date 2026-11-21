//! Purpose: shared helpers for the `sad hooks` contract-test binaries.
//! Responsibilities: build the `sad` CLI command under test, create a unique
//! throwaway git repository under the system temp directory, expose its
//! path, remove the whole tree on drop even when an assertion panics, and
//! supply the plain single-stage lefthook stub fixture.
//! Rationale: install and uninstall contract suites exercise the same
//! wiring surface over the same fixture shape, so the plumbing lives in
//! one place and each binary states only its own scenario fixtures.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_cmd::Command;

/// The `sad` binary under test, resolved through cargo.
pub fn sad() -> Command {
    Command::cargo_bin("sad").expect("sad binary builds")
}

/// A throwaway git repository under the system temp directory.
pub struct Repo(PathBuf);

impl Repo {
    /// Create an empty repository with a unique per-test label.
    pub fn init(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!("sad-hooks-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("creating fixture repository");
        let repo = Self(path);
        let status = StdCommand::new("git")
            .current_dir(repo.0.clone())
            .args(["init", "-q"])
            .status()
            .expect("git must be installed");
        assert!(status.success(), "git init must succeed");
        repo
    }

    /// The repository root, for writing configs and aiming the CLI.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A minimal lefthook config: one stage, one plain command.
pub fn lefthook_stub() -> &'static str {
    "pre-commit:\n  commands:\n    lint:\n      run: echo lint\n"
}
