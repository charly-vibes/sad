//! Purpose: CLI contract tests — the binary is the unit under test.
//! Responsibilities: pin the suite-envelope contract (JSON mode ok:true on
//! stdout, text mode short banner, bad invocations → envelope not clap text)
//! and the exit-code mirror.
//! Rationale: the envelope is the *only* output format machine callers see;
//! these tests keep the contract from drifting as subcommands land.

use assert_cmd::Command;
use predicates::str;

fn sad() -> Command {
    Command::cargo_bin("sad").expect("sad binary builds")
}

#[test]
fn version_json_emits_ok_envelope() {
    sad()
        .args(["version", "--json"])
        .assert()
        .success()
        .stdout(str::contains("\"ok\":true"))
        .stdout(str::contains("\"name\":\"sad\""));
}

#[test]
fn version_text_mode_is_a_short_banner() {
    sad()
        .arg("version")
        .assert()
        .success()
        .stdout(str::starts_with("sad "))
        .stdout(str::contains("scaffold"));
}

#[test]
fn unknown_subcommand_emits_envelope_not_clap_text() {
    sad()
        .args(["definitely-not-a-subcommand", "--json"])
        .assert()
        .failure()
        .stdout(str::contains("\"ok\":false"))
        .stdout(str::contains("argument_error"))
        .stdout(str::contains("sad --help"));
}

#[test]
fn missing_subcommand_is_an_envelope_error_not_help_text() {
    // Both bare `sad` and `sad --json` must emit the suite envelope —
    // never clap's plain-text usage — and exit non-zero.
    for args in [vec!["--json"], vec![]] {
        sad()
            .args(args)
            .assert()
            .failure()
            .stdout(str::contains("\"ok\":false"))
            .stdout(str::contains("argument_error"))
            .stdout(str::contains("subcommand"))
            .stdout(str::contains("sad --help"));
    }
}
