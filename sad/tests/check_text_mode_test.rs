//! Purpose: end-to-end contract tests for `sad check`'s one-line text mode and
//! the skip signals it appends to that line.
//! Responsibilities: pin the suffix naming undecodable sources with their
//! reason, the suffix naming skipped directories in emitted order, and the
//! unchanged bare one-liner when a tree carries no ambiguous skip.
//! Rationale: text mode is the line a human reads from a hook, so these tests
//! drive the real binary over disposable repositories instead of only the
//! rendering helpers.

mod support;

use support::envelope::*;

#[test]
fn skipped_sources_are_named_in_the_envelope_and_the_text() {
    // see sad-o40
    let repo = TempRepo::new("skipped-sources");
    repo.write("src/clean.rs", "fn clean() {}\n");
    repo.write_bytes("src/latin1.rs", b"// \xe9t\xe9\nfn latin() {}\n");
    repo.write_bytes("src/invalid.rs", b"// \xff\xfe broken\nfn broken() {}\n");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "skips alone must not fail the run: {envelope}");
    assert_eq!(envelope["ok"], true);
    assert!(
        reported_findings(&envelope).is_empty(),
        "unreadable content yields no findings: {envelope}"
    );
    assert_eq!(
        envelope["data"]["stats"]["files_skipped"], 2usize,
        "both undecodable files are counted: {envelope}"
    );

    let warnings = envelope["warnings"]
        .as_array()
        .expect("the envelope carries a warnings array");
    let mut skipped: Vec<(&str, &str)> = warnings
        .iter()
        .filter(|warning| warning["rule_name"] == "unreadable_source")
        .map(|warning| {
            (
                warning["entity_id"]
                    .as_str()
                    .expect("a skipped warning names its path"),
                warning["message"]
                    .as_str()
                    .expect("a skipped warning carries a message"),
            )
        })
        .collect();
    skipped.sort();
    assert_eq!(
        skipped.iter().map(|(path, _)| *path).collect::<Vec<&str>>(),
        vec!["src/invalid.rs", "src/latin1.rs"],
        "each skipped file is named exactly once: {envelope}"
    );
    for (path, message) in &skipped {
        assert!(
            message.contains("not valid UTF-8"),
            "`{path}` must carry the stable reason: {message}"
        );
    }

    let text = run_sad(repo.path(), &["check", "--all"]);
    assert_eq!(text.status.code(), Some(0));
    let stdout = String::from_utf8(text.stdout).expect("text stdout is UTF-8");
    assert_eq!(
        stdout.trim_end().lines().count(),
        1,
        "the skipped suffix stays on the one-line contract: {stdout}"
    );
    assert!(
        stdout.contains("pass: 0 findings in 1 file"),
        "the analysed count is unchanged: {stdout}"
    );
    assert!(
        stdout.contains("skipped 2 files"),
        "text mode counts skipped sources: {stdout}"
    );
    assert!(
        stdout.contains("src/invalid.rs (not valid UTF-8)")
            && stdout.contains("src/latin1.rs (not valid UTF-8)"),
        "text mode names every path with its reason: {stdout}"
    );
}

#[test]
fn nested_skip_list_directory_is_named_in_text_mode() {
    let repo = TempRepo::new("nested-skip-text");
    repo.copy_fixture("narration.rs", "src/target/foo.rs");
    repo.copy_fixture("narration.rs", "src/build/bar.rs");
    repo.copy_fixture("plain.rs", "src/lib.rs");

    let text = run_sad(repo.path(), &["check", "--all"]);

    assert_eq!(text.status.code(), Some(0));
    let stdout = String::from_utf8(text.stdout).expect("text stdout is UTF-8");
    assert_eq!(
        stdout.trim_end().lines().count(),
        1,
        "the skipped-directory suffix stays on the one-line contract: {stdout}"
    );
    assert!(
        stdout.contains("pass: 0 findings in 1 file"),
        "the analysed count is unchanged: {stdout}"
    );
    assert!(
        stdout.contains("skipped 2 directories: src/build/, src/target/"),
        "text mode names every skipped directory in emitted order: {stdout}"
    );

    let quiet = TempRepo::new("nested-skip-text-quiet");
    quiet.copy_fixture("plain.rs", "src/lib.rs");

    let quiet_text = run_sad(quiet.path(), &["check", "--all"]);
    assert_eq!(quiet_text.status.code(), Some(0));
    let quiet_stdout = String::from_utf8(quiet_text.stdout).expect("text stdout is UTF-8");
    assert_eq!(
        quiet_stdout.trim_end(),
        "sad check — pass: 0 findings in 1 file",
        "a tree without ambiguous skips keeps the unchanged one-liner: {quiet_stdout}"
    );
}

#[test]
fn readable_runs_carry_no_skip_signal() {
    let repo = TempRepo::new("readable-only");
    repo.write("src/clean.rs", "fn clean() {}\n");

    let (_, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(envelope["data"]["stats"]["files_skipped"], 0usize);
    let warnings = envelope["warnings"]
        .as_array()
        .expect("the envelope carries a warnings array");
    assert!(
        warnings
            .iter()
            .all(|warning| warning["rule_name"] != "unreadable_source"),
        "a readable run emits no skip warning: {envelope}"
    );

    let text = run_sad(repo.path(), &["check", "--all"]);
    let stdout = String::from_utf8(text.stdout).expect("text stdout is UTF-8");
    assert!(
        !stdout.contains("skipped"),
        "a readable run keeps the bare one-liner: {stdout}"
    );
}
