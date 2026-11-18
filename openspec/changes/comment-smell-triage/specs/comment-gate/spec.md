# comment-gate specification delta

## ADDED Requirements

### Requirement: Findings carry a smell kind and severity

Every released finding SHALL carry `smell` and `severity` fields. `smell`
SHALL be one of `narration`, `apology`, `todo`, `banner`,
`commented_out_code`, `doc_cap`, or `unclassified`. `severity` SHALL be
`error` for narration, apology, todo, and commented_out_code, and
`warning` for banner, doc_cap, and unclassified. Classification SHALL be
deterministic and documented in `docs/src/gate-model.md`.

#### Scenario: Narration restating adjacent code is an error

- **WHEN** a function body contains a comment whose distinctive tokens
  restate the next statement, or that starts with a sequencer and names
  the next call
- **THEN** the `comment_outside_channel` finding carries
  `smell: narration` and `severity: error`

#### Scenario: Apology and unmarked TODO are errors

- **WHEN** a prose comment says "for now", "temporary", "workaround",
  "hack", or contains TODO/FIXME/XXX and carries no resolvable marker
- **THEN** the finding carries `smell: apology` or `smell: todo` and
  `severity: error`

#### Scenario: Banner is a warning

- **WHEN** a prose comment body has no alphanumerics or is a separator
  run, including a separator body matched by the block check
- **THEN** the finding carries `smell: banner` and `severity: warning`,
  even when the check id is `commented_out_code`

#### Scenario: Another-language header and rationale are warnings

- **WHEN** a TypeScript or Python file opens with a `//` or `#`
  Purpose/Responsibilities header block, or a prose comment explains a
  domain fact or test scenario
- **THEN** each finding carries `smell: unclassified` and
  `severity: warning`, not an error

### Requirement: Enforcement is the baseline delta

A run SHALL block only on findings whose IDs are absent from the
baseline; ids present in `sad-baseline.json` SHALL remain grandfathered
regardless of severity. A run without a baseline SHALL report and block
on every error in the tree. Findings SHALL retain content-derived IDs, so
classification or severity changes do not churn an existing baseline.

#### Scenario: Adopted repository passes its inventory

- **WHEN** a repository adopts its current findings and reruns unchanged
- **THEN** the run has no new ids whether the tree is full of warnings or
  errors, and `stats.findings` is zero

#### Scenario: A new error blocks, a new warning does not

- **WHEN** a change adds one narration comment and one banner to an
  adopted repository
- **THEN** the run fails with `stats.findings_blocking: 1`, and the
  banner is reported in `stats.findings` without blocking

#### Scenario: Whole-tree run is the inventory

- **WHEN** `sad check --all` runs without a baseline
- **THEN** every error-severity finding in the tree is reported and the
  run exits 1

## MODIFIED Requirements

### Requirement: Channel attribution

The system SHALL parse source files with pinned tree-sitter grammars and
SHALL attribute every comment or doc-string node to exactly one class:
file-header, doc, marker, rationale, suppression, exemption, or
finding-eligible prose. A leading `SAFETY:`, `INVARIANT:`, or `SECURITY:`
token with a non-empty body SHALL be the rationale class, counted in
`stats.comments_rationale`. Attribution SHALL be purely structural (node
kind, position, leading token), never semantic.

#### Scenario: Rust file with one node per class

- **WHEN** a Rust file contains a `//!` header block, a `///` doc on a
  `pub` item, a `see sad-lcy` marker comment, a
  `// SAFETY: the lock is held` rationale, an SPDX licence block, and an
  inline narration comment
- **THEN** attribution assigns each node to header / doc / marker /
  rationale / exemption / prose respectively

#### Scenario: Rationale tag requires a body

- **WHEN** a comment reads `// SAFETY:` with no text, or
  `// NOTE: the lock is held`
- **THEN** neither is rationale; both remain finding-eligible prose

#### Scenario: TypeScript and Python parity

- **WHEN** equivalent fixtures are analyzed for TypeScript (`/** */` on
  an exported symbol, module header block comment) and Python (docstring
  on an exported symbol, module docstring)
- **THEN** attribution produces the same class assignment pattern as the
  Rust fixture

### Requirement: Prose outside channels is a finding

The system SHALL emit a `comment_outside_channel` finding with stable ID
`<path>::<symbol-path>::<content-hash>` for every comment node
attributed as finding-eligible prose, except member nodes of a block
matched by the `commented_out_code` check. Each finding SHALL carry the
`smell` and `severity` assigned by the classification requirement.
File-scoped findings (`missing_header`, `suppression_cap_exceeded`) use
`-` as the symbol-path component.

#### Scenario: Narration comment

- **WHEN** a Rust function body contains `// This function calculates
  the total`
- **THEN** a `comment_outside_channel` finding is emitted with a stable
  ID anchored to the enclosing symbol path and `severity: error`

#### Scenario: TODO stub

- **WHEN** a Python file contains `# TODO: make this faster` with no
  resolvable marker
- **THEN** a `comment_outside_channel` finding is emitted with
  `smell: todo` and `severity: error`

#### Scenario: Matched block replaces per-node prose

- **WHEN** `commented_out_code` is enabled and an adjacent run of
  finding-eligible prose nodes normalizes to a body that parses
- **THEN** one `commented_out_code` finding is emitted for the block and
  none of its member nodes emits a `comment_outside_channel` finding

#### Scenario: Predicate miss keeps per-node prose

- **WHEN** `commented_out_code` is enabled and a run's normalized body
  fails to parse as a syntactic unit
- **THEN** each finding-eligible prose node in the run still emits its
  own `comment_outside_channel` finding, with its unchanged stable ID

#### Scenario: Disabled check emits per-node prose

- **WHEN** `commented_out_code` is off and the mixed fixture from the
  block requirement runs
- **THEN** every finding-eligible prose node emits
  `comment_outside_channel` per node and no `commented_out_code`
  finding is emitted, so the file is never silent

### Requirement: Doc summary cap

The system SHALL emit a `doc_summary_cap_exceeded` finding with
`severity: warning` when a doc-channel comment's summary lead — the text
up to the first blank line — contains more than `doc_summary_cap`
sentences. Sentence count SHALL be independent of rendered line count, so
one sentence wrapped over several source lines passes. Content after the
first blank line SHALL NOT be capped. The cap SHALL be configurable in
`sad.toml` (default: 1 sentence).

#### Scenario: One-line summary with structured sections passes

- **WHEN** a Rust `pub` item carries `/// Parses config.` followed by a
  blank doc line and `# Errors` / `# Panics` sections of any length
- **THEN** no doc finding is emitted

#### Scenario: Wrapped one-sentence summary passes

- **WHEN** a Rust `pub` item carries a summary sentence wrapped over
  four `///` lines followed by a blank doc line and `# Errors` sections
- **THEN** no doc finding is emitted

#### Scenario: Two-line summary fails

- **WHEN** a docstring's summary lead contains two sentences before the
  first blank line, however the source wraps them
- **THEN** `doc_summary_cap_exceeded` is emitted with
  `severity: warning`

#### Scenario: Non-public docstrings are capped equally

- **WHEN** a private Python function carries a multi-sentence-lead
  docstring
- **THEN** `doc_summary_cap_exceeded` is emitted (the doc channel covers
  all docstrings; there is no visibility carve-out in v1)

### Requirement: Output contract mirrors pretender

WHEN a check run completes, the system SHALL emit a genesis Check
envelope with findings carrying stable IDs, `smell`, `severity`, and
remediation, plus `suppression_debt` and channel stats. `ok` and the exit
status SHALL derive from new `severity: error` findings; warnings SHALL
be reported without failing the run. `stats.findings` SHALL count every
new finding and `stats.findings_blocking` the new errors. Baseline
suppression and shrink-only adoption SHALL be unchanged.

#### Scenario: Warnings pass, errors fail

- **WHEN** a run reports only warning-severity findings
- **THEN** the envelope has `ok: true`, exit status 0, and
  `stats.findings_blocking: 0`, while the findings and warning count
  remain in `data`

#### Scenario: Error findings fail the run

- **WHEN** a run reports at least one new error-severity finding
- **THEN** the envelope has `ok: false`, exit status 1, and
  `stats.findings_blocking` equals the error count

#### Scenario: Adopt and shrink-only

- **WHEN** `sad check --adopt` runs on a tree with findings, then a
  later run fixes prose (fewer findings)
- **THEN** `sad-baseline.json` is written with the adopted IDs, the
  baseline suppresses grandfathered IDs and the later run passes;
  re-adopting a superset baseline fails the run

#### Scenario: Same command, both modes

- **WHEN** the identical `sad check` invocation runs as a pre-hook and
  in CI on the same tree
- **THEN** both produce the same envelope and exit code

#### Scenario: Remediation entry

- **WHEN** a narration, apology, banner, unclassified, or doc-cap
  finding is emitted
- **THEN** it carries a hardcoded remediation naming the structural
  dispositions for that smell (rename/extract, move to a channel, tag as
  rationale, cite a marker, add a test, or delete a banner) and never a
  bare delete for an informative comment