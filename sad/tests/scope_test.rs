//! Purpose: contract tests for `sad::scope` — commit-trailer attribution and
//! the file scope derived from it.
//! Responsibilities: pin git-trailer parsing (case-insensitive key, final
//! trailer paragraph), the governing-revision rule (last commit touching a
//! path), default versus `--all` file selection, new/uncommitted files,
//! squash-merge attributed histories, nested repo-relative paths, and the
//! `attribution.trailer` config override.
//! Rationale: default scope is the adoption guard — it must analyze new agent
//! work and skip untouched legacy prose, and attribution must come from
//! explicit commit trailers rather than git heuristics or working-tree state.

mod support;

use std::path::PathBuf;

use sad::config::Config;
use sad::scope::{governing_revisions, has_agent_trailer, select, select_in_scope};

use support::scope_repo::{Repo, paths};

#[test]
fn trailer_detection_matches_git_trailer_semantics() {
    // Scenario: Attribution comes only from an explicit commit trailer.
    assert!(has_agent_trailer(
        "fix: x\n\nAssisted-by: Claude",
        "Assisted-by"
    ));
    // Trailer keys are case-insensitive, as git treats them.
    assert!(has_agent_trailer(
        "fix: x\n\nassisted-by: Claude",
        "Assisted-by"
    ));
    assert!(!has_agent_trailer(
        "fix: x\n\nReviewed-by: Claude",
        "Assisted-by"
    ));
    // Buried inside the final paragraph is not a trailer block.
    assert!(!has_agent_trailer(
        "fix: x\n\nAssisted-by: Claude\nstill prose",
        "Assisted-by"
    ));
    // Substring keys are not the trailer key.
    assert!(!has_agent_trailer(
        "fix: x\n\nNot-Assisted-by: Claude",
        "Assisted-by"
    ));
    assert!(!has_agent_trailer("", "Assisted-by"));
}

#[test]
fn default_mode_skips_untouched_legacy_files() {
    // Scenario: Default mode skips untouched legacy files.
    let repo = Repo::init("legacy");
    repo.write_file("legacy.rs", "// legacy prose\n");
    repo.commit("add legacy file");
    repo.write_file("agent.rs", "// agent prose\n");
    repo.commit("add agent file\n\nAssisted-by: Claude");

    let cfg = Config::default();
    let selected = select(repo.path(), &paths(&["legacy.rs", "agent.rs"]), &cfg, false);

    assert_eq!(
        selected,
        paths(&["agent.rs"]),
        "default mode must analyze only agent-attributed files"
    );
}

#[test]
fn full_run_selects_every_file_in_input_order() {
    // Scenario: Full run.
    let repo = Repo::init("full");
    repo.write_file("legacy.rs", "// legacy prose\n");
    repo.commit("add legacy file");
    repo.write_file("agent.rs", "// agent prose\n");
    repo.commit("add agent file\n\nAssisted-by: Claude");

    let cfg = Config::default();
    let selected = select(repo.path(), &paths(&["agent.rs", "legacy.rs"]), &cfg, true);

    assert_eq!(
        selected,
        paths(&["agent.rs", "legacy.rs"]),
        "a full run must return every file, preserving input order"
    );
}

#[test]
fn uncommitted_new_files_are_analyzed_and_have_no_governing_revision() {
    // Scenario: Files with no commit yet are analyzed in default mode.
    let repo = Repo::init("untracked");
    repo.write_file("legacy.rs", "// legacy prose\n");
    repo.commit("legacy baseline without trailer");
    repo.write_file("untracked.rs", "// fresh agent prose\n");

    let cfg = Config::default();
    let selected = select(
        repo.path(),
        &paths(&["untracked.rs", "legacy.rs"]),
        &cfg,
        false,
    );

    assert_eq!(
        selected,
        paths(&["untracked.rs"]),
        "new files must be treated as agent-authored until committed"
    );
    let revisions = governing_revisions(repo.path(), &paths(&["untracked.rs", "legacy.rs"]));
    assert_eq!(
        revisions["untracked.rs"], None,
        "a never-committed path has no governing revision"
    );
    assert!(
        revisions["legacy.rs"].is_some(),
        "a committed path has a governing revision"
    );
}

#[test]
fn uncommitted_edits_do_not_change_attribution() {
    // Scenario: Working-tree edits leave the governing revision unchanged.
    let repo = Repo::init("edited");
    repo.write_file("edited.rs", "// legacy prose\n");
    repo.commit("legacy baseline without trailer");
    repo.write_file("edited.rs", "// legacy prose\n// agent edit\n");

    let cfg = Config::default();
    let selected = select(repo.path(), &paths(&["edited.rs"]), &cfg, false);

    assert!(
        selected.is_empty(),
        "default mode follows the governing commit, not the dirty working tree"
    );
}

#[test]
fn squash_merge_rewrites_attribution() {
    // Scenario: Squash merges produce a governing revision without a trailer.
    let repo = Repo::init("squash");
    repo.git(&["checkout", "-b", "main"]);
    repo.write_file("README.md", "baseline\n");
    repo.commit("initial baseline");

    repo.git(&["checkout", "-b", "side"]);
    repo.write_file("src/squashed.rs", "// agent prose\n");
    repo.commit("agent work\n\nAssisted-by: Claude");

    repo.git(&["checkout", "main"]);
    repo.git(&["merge", "--squash", "side"]);
    repo.commit("squash agent work without trailer");

    let cfg = Config::default();
    let selected = select(repo.path(), &paths(&["src/squashed.rs"]), &cfg, false);
    assert!(
        selected.is_empty(),
        "the latest commit after a squash has no trailer, so the file is out of scope"
    );
    assert_eq!(
        select(repo.path(), &paths(&["src/squashed.rs"]), &cfg, true),
        paths(&["src/squashed.rs"]),
        "a full run still analyzes the squash-merged file"
    );
}

#[test]
fn nested_forward_slash_paths_and_trailing_slash_roots_work() {
    // Scenario: Repo-relative nested paths keep forward slashes.
    let repo = Repo::init("nested");
    repo.write_file("src/deep/x.rs", "// agent prose\n");
    repo.commit("agent work\n\nAssisted-by: Claude");

    let cfg = Config::default();
    let files = paths(&["src/deep/x.rs"]);
    assert_eq!(
        select(repo.path(), &files, &cfg, false),
        paths(&["src/deep/x.rs"]),
        "nested repo-relative paths must match git history"
    );

    // A repository root spelled with a trailing separator must behave the same.
    let root_with_slash = PathBuf::from(format!("{}/", repo.path().display()));
    assert_eq!(
        select(&root_with_slash, &files, &cfg, false),
        paths(&["src/deep/x.rs"]),
        "a trailing slash on the repo root must not change scope"
    );
    assert!(
        governing_revisions(&root_with_slash, &paths(&["src/deep/x.rs"]))["src/deep/x.rs"]
            .is_some(),
        "a trailing slash on the repo root must not hide the governing revision"
    );
}

#[test]
fn glob_metacharacters_in_paths_resolve_the_files_own_history() {
    // Scenario: `[slug]` is a legal directory name, not a pathspec glob. A
    // later commit to the glob-matching sibling `app/blog/s/page.tsx` must not
    // become the governing revision of `app/blog/[slug]/page.tsx`.
    let repo = Repo::init("literal-pathspec");
    repo.write_file("app/blog/[slug]/page.tsx", "// agent prose\n");
    repo.commit("agent route\n\nAssisted-by: Claude");
    let agent_revision = repo.git(&["rev-parse", "HEAD"]).trim().to_string();

    repo.write_file("app/blog/s/page.tsx", "// sibling prose\n");
    repo.commit("unattributed sibling without trailer");

    let bracketed = "app/blog/[slug]/page.tsx";
    let revisions = governing_revisions(repo.path(), &paths(&[bracketed]));
    assert_eq!(
        revisions[bracketed].as_deref(),
        Some(agent_revision.as_str()),
        "a bracketed directory must resolve its own history, not a glob match"
    );

    let cfg = Config::default();
    assert_eq!(
        select(repo.path(), &paths(&[bracketed]), &cfg, false),
        paths(&[bracketed]),
        "literal-pathspec resolution keeps the attributed file in default scope"
    );
}

#[test]
fn configured_trailer_override_replaces_the_default_key() {
    // Scenario: `attribution.trailer` selects which commit trailer scopes the run.
    let repo = Repo::init("override");
    repo.write_file("assisted.rs", "// default trailer\n");
    repo.commit("default trailer\n\nAssisted-by: Claude");
    repo.write_file("session.rs", "// session trailer\n");
    repo.commit("session trailer\n\nSession: abc123");

    let both = paths(&["assisted.rs", "session.rs"]);
    let default_cfg = Config::default();
    assert_eq!(
        select(repo.path(), &both, &default_cfg, false),
        paths(&["assisted.rs"]),
        "the default trailer is Assisted-by"
    );

    let mut override_cfg = Config::default();
    override_cfg.attribution.trailer = "Session".to_string();
    assert_eq!(
        select(repo.path(), &both, &override_cfg, false),
        paths(&["session.rs"]),
        "only commits with the configured Session trailer are in scope"
    );
}

#[test]
fn selection_reports_what_attribution_scope_did() {
    let repo = Repo::init("counts");
    repo.write_file("legacy.rs", "// legacy prose\n");
    repo.commit("add legacy file");
    repo.write_file("agent.rs", "// agent prose\n");
    repo.commit("add agent file\n\nAssisted-by: Claude");
    repo.write_file("untracked.rs", "// fresh agent prose\n");

    let cfg = Config::default();
    let candidates = paths(&["legacy.rs", "agent.rs", "untracked.rs"]);
    let scoped = select_in_scope(repo.path(), &candidates, &cfg, false);

    assert_eq!(
        scoped.files,
        paths(&["agent.rs", "untracked.rs"]),
        "selection itself must not change"
    );
    assert_eq!(scoped.attributed, 1, "the trailer-attributed file");
    assert_eq!(scoped.without_commit, 1, "the never-committed file");
    assert_eq!(scoped.unattributed, 1, "the legacy file");

    let full = select_in_scope(repo.path(), &candidates, &cfg, true);
    assert_eq!(
        full.files, candidates,
        "a full run keeps every candidate in input order"
    );
    assert_eq!(
        (full.attributed, full.without_commit, full.unattributed),
        (0, 0, 0),
        "a full run resolves no attribution"
    );
}
