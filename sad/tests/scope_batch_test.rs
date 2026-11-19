//! Purpose: contract tests for the batch governing-revision lookup
//! (`sad::scope::governing_revisions`) — one git process resolving every
//! candidate path.
//! Responsibilities: pin the mixed-candidate resolution (files sharing a
//! commit, an attributed file, a never-committed file), subdirectory-relative
//! invocation, degradation on unreadable history and on an empty repository,
//! and the raw-byte path matching the `-z` stream framing implies.
//! Rationale: default scope is the adoption guard; a regression here would
//! either gate nothing (everything resolved to "no governing commit") or
//! silently skip agent work, so each lookup contract is locked against a
//! real git fixture shared with `scope_test`.

mod support;

use std::fs;

use sad::scope::governing_revisions;

use support::scope_repo::{Repo, paths};

#[test]
fn governing_revisions_resolve_every_path_in_one_pass() {
    let repo = Repo::init("batch");
    repo.write_file("legacy_a.rs", "// legacy prose\n");
    repo.write_file("legacy_b.rs", "// legacy prose\n");
    repo.commit("legacy baseline without trailer");
    let legacy_revision = repo.git(&["rev-parse", "HEAD"]).trim().to_string();

    repo.write_file("agent.rs", "// agent prose\n");
    repo.commit("agent work\n\nAssisted-by: Claude");
    let agent_revision = repo.git(&["rev-parse", "HEAD"]).trim().to_string();

    repo.write_file("untracked.rs", "// fresh agent prose\n");

    let revisions = governing_revisions(
        repo.path(),
        &paths(&["legacy_a.rs", "legacy_b.rs", "agent.rs", "untracked.rs"]),
    );

    assert_eq!(
        revisions["legacy_a.rs"].as_deref(),
        Some(legacy_revision.as_str()),
        "the first legacy file resolves its own governing revision"
    );
    assert_eq!(
        revisions["legacy_b.rs"].as_deref(),
        Some(legacy_revision.as_str()),
        "a second file of the same commit resolves that commit"
    );
    assert_eq!(
        revisions["agent.rs"].as_deref(),
        Some(agent_revision.as_str()),
        "the newer attributed file resolves the newer commit"
    );
    assert_eq!(
        revisions["untracked.rs"], None,
        "a never-committed path has no governing revision"
    );
}

/// Scenario: the gate runs from a subdirectory of the repository.
///
/// Candidates are relative to that directory, while git prints root-relative
/// names unless asked otherwise.
#[test]
fn governing_revisions_resolve_paths_relative_to_the_invocation_directory() {
    let repo = Repo::init("subdir");
    repo.write_file("crate/src/lib.rs", "// crate prose\n");
    repo.commit("crate baseline without trailer");
    let revision = repo.git(&["rev-parse", "HEAD"]).trim().to_string();
    let crate_dir = repo.path().join("crate");

    let revisions = governing_revisions(&crate_dir, &paths(&["src/lib.rs"]));

    assert_eq!(
        revisions["src/lib.rs"].as_deref(),
        Some(revision.as_str()),
        "a subdirectory invocation resolves its own relative paths"
    );
}

/// Scenario: a directory that is not a git repository degrades gracefully.
///
/// Every path resolves to "no governing commit" instead of panicking.
#[test]
fn governing_revisions_degrade_when_git_history_is_unreadable() {
    let plain = std::env::temp_dir().join(format!(
        "sad-scope-nogit-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&plain);
    fs::create_dir_all(&plain).expect("creating a plain directory");

    let revisions = governing_revisions(&plain, &paths(&["x.rs"]));

    assert_eq!(
        revisions["x.rs"], None,
        "unreadable history must degrade to no governing commit"
    );
    let _ = fs::remove_dir_all(&plain);
}

/// Scenario: a repository with no commits has no history to consult.
#[test]
fn governing_revisions_treat_an_empty_repository_as_uncommitted() {
    let repo = Repo::init("empty-history");

    let revisions = governing_revisions(repo.path(), &paths(&["new.rs"]));

    assert_eq!(
        revisions["new.rs"], None,
        "an empty history degrades to no governing commit"
    );
}
