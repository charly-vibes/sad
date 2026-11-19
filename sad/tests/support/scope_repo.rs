//! Purpose: a disposable git repository for the commit-attribution scope
//! contract tests.
//! Responsibilities: create a unique throwaway repository under the system
//! temp directory with a local identity and no signing, write files at
//! repo-relative paths, commit with verbatim messages so callers control
//! the trailer paragraph, run git, and remove the whole tree on drop even
//! when an assertion panics.
//! Rationale: governing-revision and trailer verdicts are only meaningful
//! against real git state, and a failing run must leave no state behind —
//! both scope contract binaries share this fixture so their scenarios stay
//! identical down to the temp-name uniqueness scheme.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub static NEXT_REPO: AtomicUsize = AtomicUsize::new(0);

/// A throwaway git repository under the system temp directory.
///
/// Dropping the value removes the directory even when an assertion panics,
/// so a failing run leaves no state behind.
pub struct Repo {
    path: PathBuf,
}

impl Repo {
    /// Create the repository with a local identity and signing disabled.
    pub fn init(label: &str) -> Self {
        let serial = NEXT_REPO.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sad-scope-{label}-{}-{serial}-{nanos}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("creating temp repo {}: {err}", path.display()));

        let repo = Self { path };
        repo.git(&["init", "-q"]);
        repo.git(&["config", "user.email", "sad-scope-test@example.invalid"]);
        repo.git(&["config", "user.name", "sad scope tests"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    /// The repository root, for `sad` calls that take a starting directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write `contents` at the repo-relative path, creating parents.
    pub fn write_file(&self, rel_path: &str, contents: &str) {
        let full = self.path.join(rel_path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("creating {}: {err}", parent.display()));
        }
        fs::write(&full, contents)
            .unwrap_or_else(|err| panic!("writing {}: {err}", full.display()));
    }

    /// Run git in the repo and fail loudly on a non-zero status.
    pub fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(&self.path)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .unwrap_or_else(|err| panic!("spawning `git {}`: {err}", args.join(" ")));
        assert!(
            output.status.success(),
            "`git {}` failed with {}\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        String::from_utf8(output.stdout).expect("git stdout must be valid UTF-8")
    }

    /// Stage everything and commit with `message`; the message is passed
    /// verbatim so callers control the trailer paragraph.
    pub fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "--no-verify", "-m", message]);
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Candidate path lists in the shape `scope::select` takes them.
pub fn paths(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}
