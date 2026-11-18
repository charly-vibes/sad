# comment-gate specification delta

## MODIFIED Requirements

### Requirement: Traceability markers resolve

The system SHALL resolve marker-prefixed comment IDs against
`.beads/issues.jsonl` and `openspec/specs/`, attributing the comment to
the marker channel when its ID resolves and emitting an
`unresolvable_marker` finding when it does not. `marker_prefix` and the
configured ID pattern control recognition. Doc-channel nodes SHALL never
route to the marker channel. Suppression directives SHALL be recognized
in any comment node kind. A resolver SHALL no-op while its source is
absent.

#### Scenario: Resolvable ticket ID

- **WHEN** a comment is `// see sad-lcy: cap rule comes from review`
  and `sad-lcy` exists in `.beads/issues.jsonl`, with the default marker
  knobs (`marker_prefix = see`, ID pattern
  `\b[a-z][a-z0-9]*-[a-z0-9]+\b`)
- **THEN** the comment is marker-channel; no finding is emitted

#### Scenario: Fabricated ID

- **WHEN** a comment is `// see sad-zzz: skip this for now` and
  `sad-zzz` resolves to nothing
- **THEN** an `unresolvable_marker` finding is emitted

#### Scenario: Version-number lookalike is not a marker

- **WHEN** a comment is `// requires serde-1 compat` (no marker prefix)
- **THEN** it is attributed as finding-eligible prose, not as a marker

#### Scenario: Marker prefix cannot launder a doc comment

- **WHEN** a docstring's summary lead spans three lines and begins
  `see sad-lcy:`
- **THEN** the node is doc-channel and `doc_summary_cap_exceeded` is
  emitted (the resolvable ID neither uncaps it nor converts it to a
  marker)

#### Scenario: Spec reference resolves

- **WHEN** a comment is `// see comment-gate: gate contract` and
  `.beads/issues.jsonl` exists without `comment-gate`, but
  `openspec/specs/comment-gate/` declares it
- **THEN** the comment is marker-channel; no finding is emitted

#### Scenario: Absent resolver source no-ops

- **WHEN** neither `.beads/issues.jsonl` nor `openspec/specs/` exists
- **THEN** marker resolution is skipped and no `unresolvable_marker`
  findings are emitted

#### Scenario: Suppression directives count in every comment kind

- **WHEN** a `# noqa` directive appears in a Python line comment and a
  `sad:ignore lorem trailing` directive appears inside a Rust block
  comment
- **THEN** both are recognized as suppression directives and counted in
  `suppression_debt`

### Requirement: Agent-attribution scoping

In default mode, the system SHALL analyze only files whose governing
revision carries an explicit agent-attribution marker. Files with no
commit yet SHALL be analyzed in default mode. WHEN `--all` is passed,
the system SHALL analyze every non-excluded file. Attribution SHALL come
only from explicit markers, never inferred from git heuristics.

#### Scenario: Default mode skips untouched legacy files

- **WHEN** default mode runs on a repo where only `src/new.rs` carries
  an `Assisted-by:` trailer in its governing revision (the most recent
  commit touching the file)
- **THEN** only `src/new.rs` is analyzed

#### Scenario: Full run

- **WHEN** `sad check --all` runs on the same repo
- **THEN** all non-excluded files are analyzed, attributed or not

#### Scenario: File with no commit is analyzed

- **WHEN** default mode encounters a new or uncommitted file with no
  governing revision
- **THEN** the file is analyzed (new files during agent work are treated
  as agent-authored)

#### Scenario: Configured trailer key replaces the default

- **WHEN** `attribution.trailer = "Session"` is configured and a file's
  governing revision carries a `Session:` trailer instead of
  `Assisted-by:`
- **THEN** the file is analyzed in default mode; the default key is
  not consulted

### Requirement: Output contract mirrors pretender

WHEN a check run completes, the system SHALL emit a genesis Check
envelope containing findings with stable IDs, `suppression_debt`
entries, channel and exemption stats, and a hardcoded `RemediationEntry`
per finding kind. WHEN a baseline exists, the system SHALL suppress
grandfathered findings and SHALL fail the run if the baseline would
grow. `sad check` SHALL run identically as a pre-hook and as a CI step.

#### Scenario: Adopt and shrink-only

- **WHEN** `sad check --adopt` runs on a tree with findings, then a
  later run fixes prose (fewer findings)
- **THEN** `sad-baseline.json` is written with the adopted IDs, the
  baseline suppresses grandfathered IDs and the later run passes;
  re-adopting a superset baseline fails the run

#### Scenario: Same command, both modes

- **WHEN** the identical `sad check` invocation runs as a pre-hook and
  in CI on the same tree
- **THEN** both produce the same envelope and exit code (0 = pass,
  1 = findings or gate failure)

#### Scenario: Remediation entry

- **WHEN** a `comment_outside_channel` finding is emitted
- **THEN** it carries a hardcoded remediation entry (move to a channel,
  delete, or refactor — never naming a specific refactor)