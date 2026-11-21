//! Purpose: contract tests for `sad hooks uninstall` — marker-guarded
//! removal of the sad managed block from lefthook configs.
//! Responsibilities: pin dual-stage removal of exactly what `sad hooks
//! install` wired with every other byte preserved, the no-op success when
//! no sad block exists, the ownership guard that leaves foreign managed
//! blocks untouched, and the refusal contracts — no lefthook config, a
//! repository-less directory, and unbalanced sad markers — each touching
//! nothing.
//! Rationale: uninstall edits a user-owned config file, so the contract
//! that matters is conservation: only marker-closed sad lines disappear.
//! The binary contract is locked at the CLI boundary, not the library
//! seam, mirroring the install suite.

use std::fs;

use predicates::str;

use support::hooks::{Repo, lefthook_stub, sad};

mod support;

/// A config carrying ddl-managed and ah-managed blocks alongside plain
/// stage commands: uninstall's ownership guard must leave every line of
/// this fixture byte-identical after the sad block is removed.
fn foreign_managed_config() -> &'static str {
    "pre-commit:\n  commands:\n# ddl:managed:start\n    ah-check:\n      run: ah check\n# ddl:managed:end\n    lint:\n      run: echo lint\npre-push:\n  commands:\n# ah:managed:start\n    spec-check:\n      run: ah check --mode gate\n# ah:managed:end\n    test:\n      run: echo test\n"
}

#[test]
fn uninstall_removes_the_block_from_both_stages() {
    let repo = Repo::init("unwire");
    fs::write(repo.path().join("lefthook.yml"), lefthook_stub()).unwrap();
    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success();

    sad()
        .args(["hooks", "uninstall", "--json"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("\"ok\":true"))
        .stdout(str::contains("\"pre_commit\":\"removed\""))
        .stdout(str::contains("\"pre_push\":\"removed\""));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert!(
        !content.contains("sad"),
        "no sad trace may remain; got:\n{content}"
    );
    // The stub's own lines survive byte-for-byte. The `pre-push:` scaffold
    // install added for the absent stage is not marker-owned, so uninstall
    // leaves the emptied stage in place.
    assert_eq!(
        content,
        format!("{}pre-push:\n  commands:\n", lefthook_stub()),
        "uninstall must preserve every non-sad line byte-for-byte; got:\n{content}"
    );
}

#[test]
fn uninstall_without_a_sad_block_is_a_noop_success() {
    let repo = Repo::init("noop");
    fs::write(repo.path().join("lefthook.yml"), lefthook_stub()).unwrap();

    sad()
        .args(["hooks", "uninstall", "--json"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("\"ok\":true"))
        .stdout(str::contains("\"pre_commit\":\"not_wired\""))
        .stdout(str::contains("\"pre_push\":\"not_wired\""));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert_eq!(
        content,
        lefthook_stub(),
        "a no-op must not rewrite the config; got:\n{content}"
    );
}

#[test]
fn uninstall_preserves_foreign_managed_blocks_byte_for_byte() {
    let repo = Repo::init("foreign");
    fs::write(repo.path().join("lefthook.yml"), foreign_managed_config()).unwrap();
    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success();

    let wired = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert!(
        wired.contains("# sad:managed:start"),
        "install must have wired the gate before uninstall; got:\n{wired}"
    );

    sad()
        .args(["hooks", "uninstall", "--json"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("\"ok\":true"));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert_eq!(
        content,
        foreign_managed_config(),
        "foreign managed blocks must survive byte-for-byte; got:\n{content}"
    );
}

#[test]
fn uninstall_without_lefthook_config_names_the_framework() {
    let repo = Repo::init("unwire-no-config");

    sad()
        .args(["hooks", "uninstall", "--json"])
        .current_dir(repo.path())
        .assert()
        .failure()
        .stdout(str::contains("\"ok\":false"))
        .stdout(str::contains("lefthook"));

    assert!(
        !repo.path().join("lefthook.yml").exists(),
        "uninstall must never create a config"
    );
}

/// A start marker without its end marker is refused, not half-removed:
/// uninstall never guesses where an unterminated block ends.
#[test]
fn uninstall_refuses_unbalanced_markers_without_editing() {
    let repo = Repo::init("unbalanced");
    let torn =
        "pre-commit:\n  commands:\n# sad:managed:start\n    sad-check:\n      run: sad check\n";
    fs::write(repo.path().join("lefthook.yml"), torn).unwrap();

    sad()
        .args(["hooks", "uninstall", "--json"])
        .current_dir(repo.path())
        .assert()
        .failure()
        .stdout(str::contains("\"ok\":false"))
        .stdout(str::contains("unbalanced"));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert_eq!(
        content, torn,
        "a refused run must not edit the config; got:\n{content}"
    );
}

#[test]
fn uninstall_outside_a_git_repository_writes_nothing() {
    let plain = std::env::temp_dir().join(format!("sad-hooks-plain-unwire-{}", std::process::id()));
    let _ = fs::remove_dir_all(&plain);
    fs::create_dir_all(&plain).expect("creating plain directory");

    sad()
        .args(["hooks", "uninstall", "--json"])
        .current_dir(&plain)
        .assert()
        .failure()
        .stdout(str::contains("\"ok\":false"))
        .stdout(str::contains("git repository"));

    assert!(
        !plain.join("lefthook.yml").exists(),
        "nothing may be written outside a repository"
    );
    let _ = fs::remove_dir_all(&plain);
}

/// A sad block in a stage the gate never wires (here: pre-merge) is
/// refused, not silently deleted: strip is global, outcomes are
/// stage-scoped, and uninstall refuses to mutate a file whose block it
/// cannot attribute to the pinned stages.
#[test]
fn uninstall_refuses_a_block_outside_the_pinned_stages_without_editing() {
    let repo = Repo::init("outside-stages");
    let stray = "pre-merge:\n  commands:\n# sad:managed:start\n    sad-check:\n      run: sad check\n# sad:managed:end\n    lint:\n      run: echo lint\n";
    fs::write(repo.path().join("lefthook.yml"), stray).unwrap();

    sad()
        .args(["hooks", "uninstall", "--json"])
        .current_dir(repo.path())
        .assert()
        .failure()
        .stdout(str::contains("\"ok\":false"))
        .stdout(str::contains("outside the pre-commit/pre-push stages"));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert_eq!(
        content, stray,
        "a refused run must not edit the config; got:\n{content}"
    );
}

/// The no-op path renders the uninstall command name too: `not_wired` is
/// uninstall-only vocabulary, and the first-class "nothing to unwire"
/// run must not read as `sad hooks install`.
#[test]
fn uninstall_text_mode_names_noop_stages() {
    let repo = Repo::init("unwire-noop-text");
    fs::write(repo.path().join("lefthook.yml"), lefthook_stub()).unwrap();

    sad()
        .args(["hooks", "uninstall"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("hooks uninstall"))
        .stdout(str::contains("pre-commit: not_wired"))
        .stdout(str::contains("pre-push: not_wired"));
}

#[test]
fn uninstall_text_mode_names_the_stages() {
    let repo = Repo::init("unwire-text");
    fs::write(repo.path().join("lefthook.yml"), lefthook_stub()).unwrap();
    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success();

    sad()
        .args(["hooks", "uninstall"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("hooks uninstall"))
        .stdout(str::contains("pre-commit: removed"))
        .stdout(str::contains("pre-push: removed"));
}
