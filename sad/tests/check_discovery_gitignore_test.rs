//! Purpose: end-to-end contract tests for honoring `.gitignore` during source
//! discovery inside a git work tree.
//! Responsibilities: prove an ignored generated source is never discovered or
//! gated, that a non-matching path stays gated, that negation and
//! directory-only rules behave like git, that a nested ignore file overrides a
//! shallower one, and that a tree without `.git` still walks unchanged.
//! Rationale: ignore files are the author's declared out-of-scope set, so the
//! gate must read them as tree content rather than guess generated paths by
//! name; driving the real binary keeps these tests honest about discovery,
//! selection, and the envelope, while the no-`.git` case pins the fallback that
//! keeps every pre-change repository byte-for-byte identical.

mod support;

use serde_json::Value;
use support::envelope::*;

const PROSE_SOURCE: &str = "\
export function total(items: number[]): number {
    // reduce every item into the running total
    return items.reduce((sum, item) => sum + item, 0);
}
";

const CLEAN_SOURCE: &str = "export const answer = 42;\n";

/// A disposable repository that looks like a git work tree to discovery.
///
/// Only the `.git` entry matters: discovery checks that it exists and never
/// reads inside it, so no git invocation or index is needed.
fn work_tree(tag: &str) -> TempRepo {
    let repo = TempRepo::new(tag);
    repo.write(".git/HEAD", "ref: refs/heads/main\n");
    repo
}

fn stats(envelope: &Value) -> &Value {
    &envelope["data"]["stats"]
}

#[test]
fn gitignored_sources_are_not_gated() {
    let repo = work_tree("gitignore-ignored");
    repo.write(".gitignore", "src/generated/\n");
    repo.write("src/generated/api.ts", PROSE_SOURCE);
    repo.write("src/lib.ts", CLEAN_SOURCE);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 0,
        "the ignored source cannot fail the run: {envelope}"
    );
    assert!(
        reported_findings(&envelope).is_empty(),
        "an ignored source is never judged: {envelope}"
    );
    assert_eq!(
        stats(&envelope)["files_discovered"],
        1usize,
        "the ignored source is not discovered: {envelope}"
    );
    assert_eq!(
        stats(&envelope)["files_selected"],
        1usize,
        "the ignored source is not selected: {envelope}"
    );
    assert_eq!(
        stats(&envelope)["files_analyzed"],
        1usize,
        "only the kept source is analyzed: {envelope}"
    );
}

#[test]
fn not_gated_source_in_non_ignored_path_stays_gated() {
    let repo = work_tree("gitignore-nonmatching");
    repo.write(".gitignore", "dist/\n");
    repo.write("src/generated/api.ts", PROSE_SOURCE);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 1,
        "prose outside a channel still fails the run: {envelope}"
    );
    assert_eq!(
        stats(&envelope)["files_discovered"],
        1usize,
        "a path with no matching rule stays in scope: {envelope}"
    );
    let finding = finding_of_kind(&envelope, "comment_outside_channel");
    assert_eq!(
        finding["path"], "src/generated/api.ts",
        "rule content, not path names, decides scope: {envelope}"
    );
}

#[test]
fn negated_ignore_rule_reincludes_entry() {
    let repo = work_tree("gitignore-negation");
    repo.write(".gitignore", "*.ts\n!src/generated/api.ts\n");
    repo.write("src/generated/api.ts", PROSE_SOURCE);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 1, "the re-included source is gated: {envelope}");
    let finding = finding_of_kind(&envelope, "comment_outside_channel");
    assert_eq!(finding["path"], "src/generated/api.ts");
}

#[test]
fn directory_only_rule_leaves_same_named_file() {
    let repo = work_tree("gitignore-directory-only");
    repo.write(".gitignore", "api.ts/\n");
    repo.write("src/api.ts/inside.ts", PROSE_SOURCE);
    repo.write("src/generated/api.ts", PROSE_SOURCE);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 1, "the regular file is still gated: {envelope}");
    assert_eq!(
        stats(&envelope)["files_discovered"],
        1usize,
        "only the directory named `api.ts` is pruned: {envelope}"
    );
    let findings = reported_findings(&envelope);
    assert_eq!(
        findings.len(),
        1,
        "exactly one source is judged: {envelope}"
    );
    assert_eq!(findings[0]["path"], "src/generated/api.ts");
}

#[test]
fn nested_ignore_file_overrides_root() {
    let repo = work_tree("gitignore-nested");
    repo.write(".gitignore", "src/generated/**\n");
    repo.write("src/generated/.gitignore", "!api.ts\n");
    repo.write("src/generated/api.ts", PROSE_SOURCE);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 1,
        "the nested negation re-includes the source: {envelope}"
    );
    let finding = finding_of_kind(&envelope, "comment_outside_channel");
    assert_eq!(finding["path"], "src/generated/api.ts");
}

#[test]
fn no_git_work_tree_ignores_no_ignore_file() {
    let repo = TempRepo::new("gitignore-no-work-tree");
    repo.write(".gitignore", "src/generated/\n");
    repo.write("src/generated/api.ts", PROSE_SOURCE);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 1,
        "without `.git` the old walk gates the source: {envelope}"
    );
    let finding = finding_of_kind(&envelope, "comment_outside_channel");
    assert_eq!(finding["path"], "src/generated/api.ts");
}
