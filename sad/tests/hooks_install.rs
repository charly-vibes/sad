//! Purpose: contract tests for `sad hooks install` — the lefthook managed
//! block wiring built on genesis `ensure_wired`.
//! Responsibilities: pin dual-stage wiring (pre-commit and pre-push) with the
//! gate command intact and nested inside each stage's `commands:` mapping,
//! file-level idempotence, the per-stage guard against a user-authored gate,
//! and both error contracts — a repository without a lefthook config names
//! the framework and touches nothing, and a directory outside any git
//! repository names its root and writes nothing.
//! Rationale: hook wiring is the adoption surface; a doubled gate or a write
//! outside a repository breaks every consumer's trust in the tool, so the
//! binary contract is locked at the CLI boundary, not the library seam.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_cmd::Command;
use predicates::str;

fn sad() -> Command {
    Command::cargo_bin("sad").expect("sad binary builds")
}

/// A throwaway git repository under the system temp directory.
struct Repo(PathBuf);

impl Repo {
    fn init(label: &str) -> Self {
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

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn lefthook_stub() -> &'static str {
    "pre-commit:\n  commands:\n    lint:\n      run: echo lint\n"
}

#[test]
fn install_wires_the_sad_gate_into_both_stages() {
    let repo = Repo::init("wire");
    fs::write(repo.path().join("lefthook.yml"), lefthook_stub()).unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("\"ok\":true"));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert_eq!(
        content.matches("# sad:managed:start").count(),
        2,
        "both stages must carry the managed block; got:\n{content}"
    );
    assert!(
        content
            .contains("  commands:\n# sad:managed:start\n    sad-check:\n      run: sad check\n"),
        "the gate must nest inside a stage's commands: mapping; got:\n{content}"
    );
    assert!(
        content.contains("lint"),
        "existing commands must be preserved"
    );
    let push_section = &content[content.find("pre-push:").expect("pre-push wired")..];
    assert!(
        push_section.contains("sad check"),
        "pre-push must run the gate; got:\n{content}"
    );
}

#[test]
fn install_is_idempotent() {
    let repo = Repo::init("idempotent");
    fs::write(repo.path().join("lefthook.yml"), lefthook_stub()).unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success();
    let after_first = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("\"ok\":true"));

    let after_second = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert_eq!(
        after_first, after_second,
        "a second run must not duplicate the managed block"
    );
}

#[test]
fn install_skips_a_stage_that_already_runs_sad_check() {
    let repo = Repo::init("user-gate");
    fs::write(
        repo.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n    gate:\n      run: sad check\n",
    )
    .unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success();

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    let push_at = content.find("pre-push:").unwrap_or(content.len());
    let commit_section = &content[..push_at];
    assert!(
        !commit_section.contains("# sad:managed:start"),
        "a user-authored gate must not gain a managed duplicate; got:\n{content}"
    );
    assert!(
        content.contains("pre-push:"),
        "the unwired stage must still be wired; got:\n{content}"
    );
    assert!(
        content[push_at..].contains("sad check"),
        "pre-push must run the gate; got:\n{content}"
    );
}

#[test]
fn install_without_lefthook_config_names_the_framework() {
    let repo = Repo::init("no-config");

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .failure()
        .stdout(str::contains("\"ok\":false"))
        .stdout(str::contains("lefthook"));

    assert!(
        !repo.path().join("lefthook.yml").exists(),
        "a missing config must never be created"
    );
}

#[test]
fn install_outside_a_git_repository_writes_nothing() {
    let plain = std::env::temp_dir().join(format!("sad-hooks-plain-{}", std::process::id()));
    let _ = fs::remove_dir_all(&plain);
    fs::create_dir_all(&plain).expect("creating plain directory");

    sad()
        .args(["hooks", "install", "--json"])
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

#[test]
fn install_wires_both_stages_when_both_sections_exist() {
    // genesis's file-level idempotence satisfied pre-push from
    // pre-commit's block (genesis-an6, fixed in 0.13.3); the common
    // both-sections config is the shape this command exists for.
    let repo = Repo::init("dual");
    fs::write(
        repo.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n    lint:\n      run: echo lint\npre-push:\n  commands:\n    test:\n      run: echo test\n",
    )
    .unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("\"pre_commit\":\"injected\""))
        .stdout(str::contains("\"pre_push\":\"injected\""));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert_eq!(
        content.matches("# sad:managed:start").count(),
        2,
        "both stages must carry the block; got:\n{content}"
    );
    assert!(
        content.contains("run: echo lint") && content.contains("run: echo test"),
        "existing commands must be preserved; got:\n{content}"
    );
}

#[test]
fn install_reports_partial_wiring_and_completes_idempotently() {
    // Scenario: pre-commit wires, pre-push is unanchorable (quoted key).
    // The failure envelope names pre-push while the pre-commit block stays
    // on disk; fixing the key and re-running completes without a duplicate.
    let repo = Repo::init("partial");
    fs::write(
        repo.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n    lint:\n      run: echo lint\n'pre-push':\n  commands:\n    other:\n      run: echo push\n",
    )
    .unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .failure()
        .stdout(str::contains("hooks_install_error"))
        .stdout(str::contains("pre-push"));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert!(
        content.contains("# sad:managed:start") && !content.contains("pre-push:"),
        "the wired stage must persist on disk; got:\n{content}"
    );

    fs::write(
        repo.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n# sad:managed:start\n    sad-check:\n      run: sad check\n# sad:managed:end\n    lint:\n      run: echo lint\npre-push:\n  commands:\n    other:\n      run: echo push\n",
    )
    .unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("\"pre_push\":\"injected\""));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert_eq!(
        content.matches("# sad:managed:start").count(),
        2,
        "re-running must complete wiring without duplicating the block; got:\n{content}"
    );
}

#[test]
fn install_skips_a_user_gate_after_the_managed_block() {
    // Scenario: a user adds their own `run: sad check` command below the
    // injected block; column-0 comments do not end a stage section, so the
    // guard must see it and skip the stage instead of doubling the gate.
    let repo = Repo::init("gate-after-block");
    fs::write(
        repo.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n# sad:managed:start\n    sad-check:\n      run: sad check\n# sad:managed:end\n    gate:\n      run: sad check\n",
    )
    .unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("\"pre_commit\":\"skipped_user_gate\""));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    let push_at = content.find("pre-push:").expect("pre-push must be wired");
    assert_eq!(
        content[..push_at].matches("# sad:managed:start").count(),
        1,
        "the guarded stage must not gain a duplicate block; got:\n{content}"
    );
    assert!(
        content[push_at..].contains("run: sad check"),
        "the unwired stage must still be wired; got:\n{content}"
    );
}

#[test]
fn install_text_mode_names_the_stages() {
    let repo = Repo::init("text");
    fs::write(repo.path().join("lefthook.yml"), lefthook_stub()).unwrap();

    sad()
        .args(["hooks", "install"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("hooks install"))
        .stdout(str::contains("pre-commit: injected"))
        .stdout(str::contains("pre-push: injected"));
}

#[test]
fn install_reports_partial_wiring_and_completes_idempotently() {
    // Scenario: pre-commit wires, pre-push is unanchorable (quoted key).
    // The failure envelope names pre-push while the pre-commit block stays
    // on disk; fixing the key and re-running completes without a duplicate.
    let repo = Repo::init("partial");
    fs::write(
        repo.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n    lint:\n      run: echo lint\n'pre-push':\n  commands:\n    other:\n      run: echo push\n",
    )
    .unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .failure()
        .stdout(str::contains("hooks_install_error"))
        .stdout(str::contains("pre-push"));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert!(
        content.contains("# sad:managed:start") && !content.contains("pre-push:"),
        "the wired stage must persist on disk; got:\n{content}"
    );

    fs::write(
        repo.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n# sad:managed:start\n    sad-check:\n      run: sad check\n# sad:managed:end\n    lint:\n      run: echo lint\npre-push:\n  commands:\n    other:\n      run: echo push\n",
    )
    .unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("\"pre_push\":\"injected\""));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert_eq!(
        content.matches("# sad:managed:start").count(),
        2,
        "re-running must complete wiring without duplicating the block; got:\n{content}"
    );
}

#[test]
fn install_skips_a_user_gate_after_the_managed_block() {
    // Scenario: a user adds their own `run: sad check` command below the
    // injected block; column-0 comments do not end a stage section, so the
    // guard must see it and skip the stage instead of doubling the gate.
    let repo = Repo::init("gate-after-block");
    fs::write(
        repo.path().join("lefthook.yml"),
        "pre-commit:\n  commands:\n# sad:managed:start\n    sad-check:\n      run: sad check\n# sad:managed:end\n    gate:\n      run: sad check\n",
    )
    .unwrap();

    sad()
        .args(["hooks", "install", "--json"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("\"pre_commit\":\"skipped_user_gate\""));

    let content = fs::read_to_string(repo.path().join("lefthook.yml")).unwrap();
    assert_eq!(
        content.matches("# sad:managed:start").count(),
        1,
        "the guarded stage must not gain a duplicate block; got:\n{content}"
    );
}

#[test]
fn install_text_mode_names_the_stages() {
    let repo = Repo::init("text");
    fs::write(repo.path().join("lefthook.yml"), lefthook_stub()).unwrap();

    sad()
        .args(["hooks", "install"])
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(str::contains("hooks install"))
        .stdout(str::contains("pre-commit: injected"))
        .stdout(str::contains("pre-push: injected"));
}
