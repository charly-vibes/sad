//! Purpose: end-to-end contract tests for remediation guidance, suppression
//! debt, and traceability-marker resolution in the `sad check` envelope.
//! Responsibilities: pin one stable fix description and command per finding
//! kind (with the anchor path rendered into the command), count suppression
//! directives as debt rather than findings, prove beads and openspec ids
//! resolve end to end and that each `[resolvers]` toggle is honoured, and prove
//! a corrupt beads ledger fails the gate naming the ledger.
//! Rationale: remediation and debt are the actionable surface of the envelope;
//! they must be driven through the real binary, not asserted on internals.

mod support;

use std::collections::BTreeSet;

use support::envelope::*;

/// Fixture repository whose four files yield exactly one finding per kind.
fn remediation_repo(tag: &str) -> TempRepo {
    let repo = TempRepo::new(tag);
    repo.copy_fixture("narration.rs", "src/narration.rs");
    repo.copy_fixture("doc_two_line.rs", "src/doc_two_line.rs");
    repo.copy_fixture("marker_unresolved.rs", "src/marker_unresolved.rs");
    repo.copy_fixture("missing_header.rs", "src/missing_header.rs");
    repo.write_beads_ledger(LEDGER_ID);
    repo.write("sad.toml", "require_header = true\n");
    repo
}

/// Finding kinds the remediation fixture repository must report, one each.
const EXPECTED_KINDS: [&str; 4] = [
    "comment_outside_channel",
    "doc_summary_cap_exceeded",
    "unresolvable_marker",
    "missing_header",
];

#[test]
fn remediation_is_per_kind() {
    // spec: Remediation / every finding kind carries its own stable, deterministic fix guidance.
    let repo = remediation_repo("remediation_kinds");
    let (code, first) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "four findings must fail the run: {first}");
    assert_eq!(
        reported_findings(&first).len(),
        EXPECTED_KINDS.len(),
        "exactly one finding per kind: {first}"
    );
    for check in EXPECTED_KINDS {
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

    let descriptions: Vec<&str> = EXPECTED_KINDS
        .iter()
        .map(|check| remediation_description(finding_of_kind(&first, check)))
        .collect();
    let unique: BTreeSet<&str> = descriptions.iter().copied().collect();
    assert_eq!(
        unique.len(),
        EXPECTED_KINDS.len(),
        "remediation descriptions must be per kind, not shared: {descriptions:?}"
    );
}

#[test]
fn remediation_descriptions_name_the_fix() {
    // spec: Remediation / each hardcoded description names the concrete remedy for its kind.
    let repo = remediation_repo("remediation_text");
    let (code, first) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "four findings must fail the run: {first}");

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
}

#[test]
fn remediation_commands_render_the_anchor_and_stay_deterministic() {
    // spec: Remediation / commands render the finding's anchor path and are stable across runs.
    let repo = remediation_repo("remediation_commands");
    let (code, first) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 1, "four findings must fail the run: {first}");

    let commands = remediation_commands(&first);
    for finding in reported_findings(&first) {
        let path = finding["path"].as_str().expect("finding path is a string");
        let command = finding["remediation"]["command"]
            .as_str()
            .expect("remediation command is a string");
        assert!(
            command.contains(path),
            "`{path}` must be rendered into its own remediation command: {command}"
        );
        assert!(
            !command.contains("<path>"),
            "no `<path>` placeholder may survive into the envelope: {command}"
        );
        assert!(
            !command.contains('<') && !command.contains('>'),
            "remediation commands must be runnable as emitted: {command}"
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
fn a_disabled_beads_resolver_never_reads_the_ledger() {
    // spec: Traceability markers / a disabled resolver counts as an absent source.
    let repo = TempRepo::new("resolver-beads-off");
    repo.copy_fixture("markers.rs", "src/markers.rs");
    repo.write_beads_ledger(LEDGER_ID);
    repo.write("sad.toml", "[resolvers]\nbeads = false\n");

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        code, 0,
        "a disabled beads resolver makes marker judgement a no-op: {envelope}"
    );
    assert_eq!(envelope["ok"], true);
    assert!(
        reported_findings(&envelope).is_empty(),
        "the ledger ids must not resolve while the resolver is off: {envelope}"
    );
}

#[test]
fn the_openspec_resolver_satisfies_spec_ids_only_while_enabled() {
    // spec: Traceability markers / ids declared by openspec specs resolve.
    let repo = TempRepo::new("resolver-openspec");
    repo.write(
        "src/marker.rs",
        "// see comment-gate: the contract this file implements\n",
    );
    repo.write("openspec/specs/comment-gate/spec.md", "# comment-gate\n");
    repo.write_beads_ledger(LEDGER_ID);

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);
    assert_eq!(code, 0, "the spec id resolves against openspec: {envelope}");
    assert!(
        reported_findings(&envelope).is_empty(),
        "a resolvable spec id is not a finding: {envelope}"
    );

    repo.write("sad.toml", "[resolvers]\nopenspec = false\n");
    let (disabled_code, disabled) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(
        disabled_code, 1,
        "without openspec the spec id resolves to nothing: {disabled}"
    );
    let markers = reported_findings(&disabled);
    assert_eq!(markers.len(), 1, "one unresolved marker: {disabled}");
    assert_eq!(markers[0]["check"], "unresolvable_marker");
}

#[test]
fn a_corrupt_beads_ledger_fails_the_gate_naming_the_ledger() {
    // spec: Output contract / a broken resolver source fails naming the artifact, never the comments.
    let repo = TempRepo::new("corrupt-ledger");
    repo.copy_fixture("markers.rs", "src/markers.rs");
    repo.write(
        ".beads/issues.jsonl",
        "{\"title\":\"truncated\"\nnot json\n",
    );

    let (code, envelope) = run_json(repo.path(), &["check", "--all", "--json"]);

    assert_eq!(code, 1, "a broken ledger must fail the run: {envelope}");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"]["code"], "ledger_error");
    let message = envelope["data"]["message"]
        .as_str()
        .expect("error message is a string");
    assert!(
        message.contains(".beads/issues.jsonl"),
        "the gate error must name the ledger, not a comment: {message}"
    );
    let remediation = envelope["data"]["remediation"]
        .as_array()
        .expect("error remediation is an array");
    assert!(
        !remediation.is_empty(),
        "Invariant 3.2.5: the error carries remediation: {envelope}"
    );
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
