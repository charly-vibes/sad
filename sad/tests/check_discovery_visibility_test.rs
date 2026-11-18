//! Purpose: end-to-end contract tests for what discovery skips and what it
//! must name instead of skipping silently.
//! Responsibilities: prove a nested skip-list directory produces one
//! `skipped_source_directory` warning naming the path and the skip-list name,
//! that a root-level build directory stays silent, and that the warning never
//! changes the verdict or the analyzed count.
//! Rationale: a skipped directory hides whatever sources it holds, so the
//! ambiguous-nested case is a contract with consumers: the run either judges a
//! source or says why it did not. Root-level build output is the conventional
//! case, so it stays quiet to keep every ordinary repository warning-free.

mod support;

use support::envelope::*;

#[test]
fn nested_skip_list_directory_is_named() {
    let repo = TempRepo::new("ambiguous-visibility");
    repo.copy_fixture("narration.rs", "src/target/foo.rs");
    repo.copy_fixture("plain.rs", "src/lib.rs");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 0, "a warning never fails the run: {envelope}");
    assert_eq!(envelope["ok"], true);
    assert!(
        reported_findings(&envelope).is_empty(),
        "the skipped source is never analysed: {envelope}"
    );
    assert_eq!(
        envelope["data"]["stats"]["files_discovered"], 1usize,
        "only the non-skipped source is discovered: {envelope}"
    );
    assert_eq!(
        envelope["data"]["stats"]["files_analyzed"], 1usize,
        "only the non-skipped source is analysed: {envelope}"
    );

    let warnings = envelope["warnings"]
        .as_array()
        .unwrap_or_else(|| panic!("the envelope must carry warnings: {envelope}"));
    assert_eq!(
        warnings.len(),
        1,
        "exactly one skipped directory is named: {envelope}"
    );
    let warning = &warnings[0];
    assert_eq!(
        warning["rule_name"], "skipped_source_directory",
        "the warning names the discovery rule: {warning}"
    );
    assert_eq!(
        warning["entity_id"], "src/target/",
        "the warning names the skipped directory, with a trailing slash: {warning}"
    );
    let message = warning["message"]
        .as_str()
        .unwrap_or_else(|| panic!("the warning carries a message: {warning}"));
    assert!(
        message.contains("src/target/") && message.contains("`target`"),
        "the message names the path and the skip-list name: {message}"
    );
    let remediation = warning["suggested_remediation"]
        .as_str()
        .unwrap_or_else(|| panic!("the warning carries a remediation: {warning}"));
    assert!(
        remediation.contains("Rename") && remediation.contains("build output"),
        "the remediation offers rename or keep-for-build-output: {remediation}"
    );
}

#[test]
fn root_build_output_stays_silent() {
    let repo = TempRepo::new("conventional-build");
    repo.copy_fixture("narration.rs", "target/foo.rs");
    repo.copy_fixture("plain.rs", "src/lib.rs");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 0, "a root build tree cannot fail the run: {envelope}");
    assert_eq!(envelope["ok"], true);
    let warnings = envelope["warnings"]
        .as_array()
        .unwrap_or_else(|| panic!("the envelope must carry a warnings array: {envelope}"));
    assert!(
        warnings.is_empty(),
        "a root-level `target/` is conventional build output: {envelope}"
    );
    assert_eq!(
        envelope["data"]["stats"]["files_discovered"], 1usize,
        "the root build tree is never walked: {envelope}"
    );
    assert_eq!(
        envelope["data"]["stats"]["files_analyzed"], 1usize,
        "the non-skipped source is analysed: {envelope}"
    );
    assert!(
        reported_findings(&envelope).is_empty(),
        "the skipped source yields no findings: {envelope}"
    );
}
