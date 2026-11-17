//! Purpose: end-to-end contract tests for the `sad check` envelope — the
//! binary, its JSON body, text rendering, remediation, and exit status.
//! Responsibilities: pin findings, suppression debt, exemptions/excludes,
//! marker resolution, commit scoping, determinism, stats shape, and CI parity
//! against disposable fixture repositories.
//! Rationale: the envelope is the only machine contract; driving the real
//! binary keeps the tests honest about file discovery, config resolution, git
//! scope, and exit codes rather than only the pure check functions.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::atomic::{AtomicU64, Ordering};

use assert_cmd::Command;
use serde_json::Value;

/// Static inputs copied into each dynamic fixture repository.
const FIXTURES: &str = "tests/fixtures/envelope";
/// Fabricated beads id used by fixtures that need a resolvable marker.
const LEDGER_ID: &str = "sad-lcy";

/// Unique per-test scratch repository under the system temp dir; removed on
/// drop so no test depends on or leaves workspace state.
struct TempRepo(PathBuf);

impl TempRepo {
    fn new(tag: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("sad-envelope-{tag}-{}-{id}", std::process::id()));
        // Start empty even if a crashed run left the same name behind.
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp repo is created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// Copy a static fixture into the repo at a repo-relative destination.
    fn copy_fixture(&self, fixture: &str, rel: &str) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURES)
            .join(fixture);
        let bytes = std::fs::read(&source)
            .unwrap_or_else(|err| panic!("fixture {} must be readable: {err}", source.display()));
        let destination = self.0.join(rel);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).expect("fixture parent directory is created");
        }
        std::fs::write(&destination, bytes).unwrap_or_else(|err| {
            panic!("fixture {} must be copied: {err}", destination.display())
        });
    }

    fn write(&self, rel: &str, contents: &str) {
        let destination = self.0.join(rel);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).expect("parent directory is created");
        }
        std::fs::write(&destination, contents)
            .unwrap_or_else(|err| panic!("{} must be writable: {err}", destination.display()));
    }

    fn write_beads_ledger(&self, id: &str) {
        self.write(".beads/issues.jsonl", &format!("{{\"id\":\"{id}\"}}\n"));
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sad() -> Command {
    Command::cargo_bin("sad").expect("sad binary builds")
}

fn run_sad(repo: &Path, args: &[&str]) -> Output {
    sad()
        .current_dir(repo)
        .args(args)
        .output()
        .expect("sad runs")
}

/// Run the binary in `repo` and parse its single JSON envelope from stdout.
fn run_json(repo: &Path, args: &[&str]) -> (i32, Value) {
    let output = run_sad(repo, args);
    let stdout = String::from_utf8(output.stdout).expect("envelope stdout is UTF-8");
    let envelope = serde_json::from_str(stdout.trim()).unwrap_or_else(|err| {
        panic!(
            "stdout must be one JSON envelope: {err}\nstdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    let code = output
        .status
        .code()
        .expect("sad exits with an explicit status");
    (code, envelope)
}

fn reported_findings(envelope: &Value) -> &[Value] {
    envelope["data"]["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("check envelope must carry a findings array: {envelope}"))
        .as_slice()
}

fn finding_of_kind<'a>(envelope: &'a Value, check: &str) -> &'a Value {
    reported_findings(envelope)
        .iter()
        .find(|finding| finding["check"].as_str() == Some(check))
        .unwrap_or_else(|| panic!("expected a `{check}` finding, got: {envelope}"))
}

fn remediation_description(finding: &Value) -> &str {
    finding["remediation"]["description"]
        .as_str()
        .unwrap_or_else(|| panic!("finding must carry a remediation description: {finding}"))
}

/// `(check id, remediation command)` pairs in a stable order.
fn remediation_commands(envelope: &Value) -> Vec<(String, String)> {
    let mut commands: Vec<(String, String)> = reported_findings(envelope)
        .iter()
        .map(|finding| {
            (
                finding["check"]
                    .as_str()
                    .expect("check id is a string")
                    .to_string(),
                finding["remediation"]["command"]
                    .as_str()
                    .expect("remediation command is a string")
                    .to_string(),
            )
        })
        .collect();
    commands.sort();
    commands
}

fn git(repo: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=sad envelope tests",
            "-c",
            "user.email=sad-envelope@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .status()
        .expect("git is available");
    assert!(
        status.success(),
        "git {args:?} failed in {}",
        repo.display()
    );
}

fn commit_file(repo: &Path, file: &str, subject: &str, trailer: Option<&str>) {
    git(repo, &["add", "--", file]);
    match trailer {
        Some(trailer) => git(repo, &["commit", "--quiet", "-m", subject, "-m", trailer]),
        None => git(repo, &["commit", "--quiet", "-m", subject]),
    }
}

#[test]
fn findings_fail_the_run_with_a_stable_id_and_remediation() {
    // spec: Check envelope / a prose finding fails the run with a repo-relative anchor and remediation.
    let repo = TempRepo::new("findings");
    repo.copy_fixture("narration.rs", "src/lib.rs");
    repo.write("sad.toml", "require_header = true\n");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 1, "a findings-bearing check must exit 1: {envelope}");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["envelope_kind"], "check");

    let reported = reported_findings(&envelope);
    assert_eq!(
        reported.len(),
        1,
        "only the narration comment is a finding: {envelope}"
    );
    let finding = &reported[0];
    assert_eq!(finding["check"], "comment_outside_channel");
    assert_eq!(finding["path"], "src/lib.rs");
    assert_eq!(finding["symbol_path"], "helper");
    assert_eq!(finding["line"], 6usize);

    let id = finding["id"].as_str().expect("finding id is a string");
    let id_shape =
        regex::Regex::new(r"^src/lib\.rs::.+::[0-9a-f]{16}$").expect("id pattern compiles");
    assert!(
        id_shape.is_match(id),
        "finding id must be path::symbol::hash: {id}"
    );
    assert!(
        id.starts_with("src/lib.rs::helper::"),
        "finding id must name the file and symbol: {id}"
    );

    assert!(
        !finding["message"]
            .as_str()
            .expect("message is a string")
            .trim()
            .is_empty()
    );
    assert!(
        !finding["remediation"]["command"]
            .as_str()
            .expect("remediation command is a string")
            .trim()
            .is_empty()
    );
    assert!(!remediation_description(finding).trim().is_empty());
}

#[test]
fn a_clean_run_passes() {
    // spec: Check envelope / header, doc, and resolvable marker channels yield a passing run.
    let repo = TempRepo::new("clean");
    repo.copy_fixture("clean.rs", "src/lib.rs");
    repo.write_beads_ledger(LEDGER_ID);

    let (code, envelope) = run_json(repo.path(), &["check", "--json"]);

    assert_eq!(code, 0);
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["envelope_kind"], "check");
    assert!(
        reported_findings(&envelope).is_empty(),
        "clean run must report no findings: {envelope}"
    );
    assert_eq!(envelope["data"]["stats"]["findings"], 0usize);
    assert_eq!(envelope["data"]["stats"]["files_analyzed"], 1usize);
}

#[test]
fn remediation_is_per_kind_and_hardcoded() {
    // spec: Remediation / every finding kind carries its own stable, deterministic fix guidance.
    let repo = TempRepo::new("remediation");
    repo.copy_fixture("narration.rs", "src/narration.rs");
    repo.copy_fixture("doc_two_line.rs", "src/doc_two_line.rs");
    repo.copy_fixture("marker_unresolved.rs", "src/marker_unresolved.rs");
    repo.copy_fixture("missing_header.rs", "src/missing_header.rs");
    repo.write_beads_ledger(LEDGER_ID);
    repo.write("sad.toml", "require_header = true\n");

    let (code, first) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "four findings must fail the run: {first}");

    let expected = [
        "comment_outside_channel",
        "doc_summary_cap_exceeded",
        "unresolvable_marker",
        "missing_header",
    ];
    assert_eq!(
        reported_findings(&first).len(),
        expected.len(),
        "exactly one finding per kind: {first}"
    );
    for check in expected {
        let finding = finding_of_kind(&first, check);
        assert!(
            !finding["remediation"]["command"]
                .as_str()
                .expect("remediation command is a string")
                .trim()
                .is_empty(),
            "`{check}` must carry a remediation command: {finding}"
        );
        assert!(
            !remediation_description(finding).trim().is_empty(),
            "`{check}` must carry a remediation description: {finding}"
        );
    }

    let descriptions: Vec<&str> = expected
        .iter()
        .map(|check| remediation_description(finding_of_kind(&first, check)))
        .collect();
    let unique: BTreeSet<&str> = descriptions.iter().copied().collect();
    assert_eq!(
        unique.len(),
        expected.len(),
        "remediation descriptions must be per kind, not shared: {descriptions:?}"
    );

    let outside = remediation_description(finding_of_kind(&first, "comment_outside_channel"))
        .to_ascii_lowercase();
    assert!(
        outside.contains("channel"),
        "outside-channel fix must mention the channel: {outside}"
    );
    assert!(
        outside.contains("move") || outside.contains("delete"),
        "outside-channel fix must say move-or-delete: {outside}"
    );

    let doc = remediation_description(finding_of_kind(&first, "doc_summary_cap_exceeded"))
        .to_ascii_lowercase();
    assert!(
        doc.contains("summary") && doc.contains("cap"),
        "doc-cap fix must mention the summary lead and the cap: {doc}"
    );

    let marker = remediation_description(finding_of_kind(&first, "unresolvable_marker"))
        .to_ascii_lowercase();
    assert!(
        marker.contains("resolve"),
        "marker fix must say resolve: {marker}"
    );
    assert!(
        marker.contains("id"),
        "marker fix must name the id: {marker}"
    );
    assert!(
        marker.contains("remove") || marker.contains("delete"),
        "marker fix must say remove-or-delete: {marker}"
    );

    let header =
        remediation_description(finding_of_kind(&first, "missing_header")).to_ascii_lowercase();
    assert!(
        header.contains("header"),
        "header fix must mention the header: {header}"
    );
    assert!(
        header.contains("purpose") && header.contains("rationale"),
        "header fix must name the required keywords: {header}"
    );

    let commands = remediation_commands(&first);
    for (_, command) in &commands {
        let without_path_argument = command.replace("<path>", "");
        assert!(
            !without_path_argument.contains('<') && !without_path_argument.contains('>'),
            "remediation commands may only use a <path> placeholder: {command}"
        );
    }

    let (second_code, second) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(second_code, code);
    assert_eq!(
        commands,
        remediation_commands(&second),
        "remediation commands must be deterministic across runs"
    );
}

#[test]
fn suppression_debt_is_reported_not_a_finding() {
    // spec: Suppression debt / every directive is counted with path, line, form, and scope.
    let repo = TempRepo::new("suppression");
    repo.copy_fixture("noqa.py", "probe.py");

    let (code, envelope) = run_json(repo.path(), &["check", "--json"]);

    assert_eq!(code, 0);
    assert_eq!(envelope["ok"], true);
    assert!(
        reported_findings(&envelope).is_empty(),
        "suppression debt is counted, not flagged: {envelope}"
    );

    let debt = envelope["data"]["suppression_debt"]
        .as_array()
        .expect("suppression_debt is an array");
    assert_eq!(debt.len(), 2, "both directives are counted: {envelope}");
    assert_eq!(debt[0]["path"], "probe.py");
    assert_eq!(debt[0]["line"], 1usize);
    assert_eq!(debt[0]["form"], "noqa");
    assert!(
        debt[0]["check"].is_null(),
        "an unscoped noqa has no check: {}",
        debt[0]
    );
    assert_eq!(debt[1]["path"], "probe.py");
    assert_eq!(debt[1]["line"], 2usize);
    assert_eq!(debt[1]["form"], "sad_ignore");
    assert_eq!(debt[1]["check"], "comment_outside_channel");
    assert_eq!(envelope["data"]["stats"]["suppression_debt"], 2usize);
}

#[test]
fn exemptions_and_excludes_keep_files_out_of_analysis() {
    // spec: Exemptions and excludes / generated files and excluded globs are counted but not analyzed.
    let repo = TempRepo::new("exemptions");
    repo.copy_fixture("narration.rs", "vendor/x.rs");
    repo.copy_fixture("generated.rs", "generated.rs");
    repo.write("sad.toml", "exclude = [\"vendor/**\"]\n");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 0,
        "excluded and exempt files cannot fail the run: {envelope}"
    );
    assert_eq!(envelope["ok"], true);
    assert_eq!(
        envelope["data"]["stats"]["files_analyzed"], 0usize,
        "neither the excluded nor the exempt file is analyzed: {envelope}"
    );
    assert!(
        envelope["data"]["stats"]["files_exempt"]
            .as_u64()
            .expect("files_exempt is a count")
            >= 1,
        "the generated file is counted as exempt: {envelope}"
    );
    assert!(
        reported_findings(&envelope).is_empty(),
        "excluded files yield no findings: {envelope}"
    );
    for finding in reported_findings(&envelope) {
        assert!(
            !finding["path"]
                .as_str()
                .unwrap_or("")
                .starts_with("vendor/"),
            "excluded files must never be reported: {finding}"
        );
    }
}

#[test]
fn marker_resolution_is_end_to_end() {
    // spec: Traceability markers / beads ids resolve, unknown ids fail, and an absent ledger is a no-op.
    let repo = TempRepo::new("markers");
    repo.copy_fixture("markers.rs", "src/markers.rs");
    repo.write_beads_ledger(LEDGER_ID);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 1,
        "an unresolved marker must fail the run: {envelope}"
    );
    let markers = reported_findings(&envelope);
    assert_eq!(
        markers.len(),
        1,
        "only the unknown id is a finding: {envelope}"
    );
    assert_eq!(markers[0]["check"], "unresolvable_marker");
    let text = markers[0]["text"]
        .as_str()
        .expect("finding text is a string");
    assert!(
        text.contains("sad-nope"),
        "the unresolved marker is reported: {text}"
    );
    assert!(
        !text.contains("sad-lcy"),
        "the resolved marker is not reported: {text}"
    );

    std::fs::remove_dir_all(repo.path().join(".beads")).expect("beads ledger is removed");
    let (clean_code, clean) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(
        clean_code, 0,
        "an absent ledger makes marker resolution a no-op: {clean}"
    );
    assert!(
        reported_findings(&clean).is_empty(),
        "no ledger means no marker findings: {clean}"
    );
}

#[test]
fn scope_uses_commit_attribution() {
    // spec: Scope / the default run analyzes only Assisted-by commits while --all analyzes everything.
    let repo = TempRepo::new("scope");
    repo.copy_fixture("plain.rs", "attributed.rs");
    repo.copy_fixture("plain.rs", "legacy.rs");
    git(repo.path(), &["init", "--quiet"]);
    commit_file(
        repo.path(),
        "attributed.rs",
        "add attributed file",
        Some("Assisted-by: Claude"),
    );
    commit_file(repo.path(), "legacy.rs", "add legacy file", None);

    let (code, scoped) = run_json(repo.path(), &["check", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(scoped["ok"], true);
    assert_eq!(
        scoped["data"]["stats"]["files_analyzed"], 1usize,
        "only the attributed file is in scope: {scoped}"
    );

    let (all_code, all) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(all_code, 0);
    assert_eq!(
        all["data"]["stats"]["files_analyzed"], 2usize,
        "--all ignores commit attribution: {all}"
    );
}

#[test]
fn repeated_runs_are_deterministic() {
    // spec: Determinism / the same tree yields identical data across runs.
    let repo = TempRepo::new("determinism");
    repo.copy_fixture("narration.rs", "src/lib.rs");
    repo.write("sad.toml", "require_header = true\n");

    let (first_code, first) = run_json(repo.path(), &["check", "--all", "--json"]);
    let (second_code, second) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(first_code, second_code);
    assert_eq!(
        first["data"], second["data"],
        "check data must be deterministic: first={first} second={second}"
    );
    for finding in reported_findings(&first) {
        let path = finding["path"].as_str().expect("path is a string");
        assert!(
            !path.starts_with('/'),
            "finding paths must be repo-relative: {path}"
        );
        assert!(
            !path.contains("sad-envelope-"),
            "finding paths must not leak the temp root: {path}"
        );
    }
}

#[test]
fn stats_shape_reports_every_class() {
    // spec: Stats / every stat key is present and class counts equal the fixture's node classes.
    let repo = TempRepo::new("stats");
    repo.copy_fixture("all_classes.py", "classes.py");

    let (code, envelope) = run_json(repo.path(), &["check", "--json"]);
    assert_eq!(code, 1, "the one prose node is a finding: {envelope}");

    let stats = &envelope["data"]["stats"];
    for key in [
        "files_analyzed",
        "files_exempt",
        "exempt_nodes",
        "comments_header",
        "comments_doc",
        "comments_marker",
        "comments_suppression",
        "comments_prose",
        "suppression_debt",
        "findings",
    ] {
        assert!(
            stats.get(key).and_then(Value::as_u64).is_some(),
            "stats.{key} must be a non-negative count: {stats}"
        );
    }

    assert_eq!(stats["files_analyzed"], 1usize);
    assert_eq!(stats["files_exempt"], 0usize);
    assert_eq!(stats["exempt_nodes"], 1usize);
    assert_eq!(stats["comments_header"], 1usize);
    assert_eq!(stats["comments_doc"], 1usize);
    assert_eq!(stats["comments_marker"], 1usize);
    assert_eq!(stats["comments_suppression"], 1usize);
    assert_eq!(stats["comments_prose"], 1usize);
    assert_eq!(stats["suppression_debt"], 1usize);
    assert_eq!(stats["findings"], reported_findings(&envelope).len() as u64);
}

#[test]
fn text_mode_reports_the_verdict() {
    // spec: Text mode / the human line names the verdict and the finding count.
    let findings_repo = TempRepo::new("text-findings");
    findings_repo.copy_fixture("narration.rs", "src/lib.rs");
    findings_repo.write("sad.toml", "require_header = true\n");

    let failing = run_sad(findings_repo.path(), &["check", "--all"]);
    assert_eq!(failing.status.code(), Some(1));
    let failing_stdout = String::from_utf8(failing.stdout).expect("text stdout is UTF-8");
    assert!(
        failing_stdout.contains("fail"),
        "text mode must say fail: {failing_stdout}"
    );
    assert!(
        failing_stdout
            .lines()
            .any(|line| line.contains("fail") && line.contains('1')),
        "text mode must carry the finding count: {failing_stdout}"
    );

    let clean_repo = TempRepo::new("text-clean");
    clean_repo.copy_fixture("clean.rs", "src/lib.rs");
    clean_repo.write_beads_ledger(LEDGER_ID);

    let passing = run_sad(clean_repo.path(), &["check", "--all"]);
    assert_eq!(passing.status.code(), Some(0));
    let passing_stdout = String::from_utf8(passing.stdout).expect("text stdout is UTF-8");
    assert!(
        passing_stdout.contains("pass"),
        "text mode must say pass: {passing_stdout}"
    );
}

#[test]
fn hooks_and_ci_see_identical_results() {
    // spec: CI parity / repeated invocations over the same tree match in data and exit status.
    let repo = TempRepo::new("parity");
    repo.copy_fixture("narration.rs", "src/lib.rs");
    repo.write("sad.toml", "require_header = true\n");

    let (hook_code, hook) = run_json(repo.path(), &["check", "--all", "--json"]);
    let (ci_code, ci) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        hook_code, ci_code,
        "exit status must match across invocations"
    );
    assert_eq!(hook["ok"], ci["ok"], "ok must match across invocations");
    assert_eq!(
        hook["data"], ci["data"],
        "data must match across invocations: hook={hook} ci={ci}"
    );
}
