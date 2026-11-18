# comment-gate specification delta

## ADDED Requirements

### Requirement: Findings carry severity

Every released finding SHALL carry `severity`, either `error` or
`warning`. `error` SHALL apply to narration, apology, todo,
`missing_header`, `unresolvable_marker`, and `suppression_cap_exceeded`;
`warning` SHALL apply to banner, unclassified, `doc_summary_cap_exceeded`,
and `commented_out_code` until the precision change promotes the block
finding. `smell` SHALL be null for findings not derived from comment
prose.

#### Scenario: Structural findings are errors with a null smell

- **WHEN** a run emits `missing_header`, `unresolvable_marker`, or
  `suppression_cap_exceeded`
- **THEN** each carries `smell: null` and `severity: error`

#### Scenario: Block findings are warnings until precision lands

- **WHEN** the block predicate matches a non-banner body
- **THEN** the `commented_out_code` finding carries
  `smell: commented_out_code` and `severity: warning`

#### Scenario: Doc cap is a warning

- **WHEN** a doc summary exceeds the sentence cap
- **THEN** the `doc_summary_cap_exceeded` finding carries
  `smell: doc_cap` and `severity: warning`

### Requirement: Smell names the prose smell

A finding derived from comment prose SHALL carry `smell` in `narration`,
`apology`, `todo`, `banner`, `commented_out_code`, `doc_cap`, or
`unclassified`. Classification SHALL run once after all checks, on each
maximal adjacent prose run, and SHALL apply to every member node while
IDs remain per node. The rules SHALL be deterministic and documented in
`docs/src/gate-model.md`.

#### Scenario: Narration rule is mechanical

- **WHEN** a prose run's distinctive tokens (identifiers or literals of
  length >= 4 outside the stopword set) match at least two tokens of the
  next non-empty statement, or the run starts with a sequencer and names
  the next call
- **THEN** every member node carries `smell: narration` and
  `severity: error`

#### Scenario: Apology requires an apology shape

- **WHEN** a prose run contains `for now`, `temporary fix`, `workaround`,
  `hack:`, `legacy shim`, or `to be removed`, and no resolvable marker
- **THEN** it carries `smell: apology` and `severity: error`; a bare
  "temporary" or "hack" is not enough

#### Scenario: Multi-line prose is judged as one run

- **WHEN** a narration sentence is split across three adjacent `//` nodes
- **THEN** all three findings carry `smell: narration` and
  `severity: error`, each with its own stable ID

#### Scenario: Module-level narration is in scope

- **WHEN** a script's top-level statement is preceded by a code-restating
  comment outside any function body
- **THEN** the finding is classified by the same narration rule

#### Scenario: Ambiguous prose is a warning

- **WHEN** a prose run matches no smell rule, or has no next statement to
  compare against
- **THEN** it carries `smell: unclassified` and `severity: warning`

### Requirement: Enforcement is the baseline delta

A run SHALL block only on new findings; ids present in
`sad-baseline.json` SHALL remain grandfathered regardless of severity. A
run without a baseline SHALL report every finding. Adoption SHALL include
warnings and errors. Content-derived IDs SHALL NOT encode severity, so a
classifier change does not churn the baseline; a severity change to an
adopted id stays grandfathered until re-adoption.

#### Scenario: Adopted repository passes its inventory

- **WHEN** a repository adopts its current findings and reruns unchanged
- **THEN** `stats.findings` is zero whether the tree held warnings or
  errors

#### Scenario: A new error blocks, a new warning does not

- **WHEN** a change adds one narration comment and one banner to an
  adopted repository
- **THEN** the run fails with `stats.findings: 2` and
  `stats.findings_blocking: 1`

#### Scenario: Whole-tree run is the inventory

- **WHEN** `sad check --all` runs without a baseline
- **THEN** every finding in the tree is reported and the run exits 1 only
  when an error-severity finding exists

#### Scenario: Severity drift is grandfathered

- **WHEN** an adopted warning becomes an error after a classifier change
- **THEN** its id is still suppressed and the run does not fail until the
  baseline is re-adopted

### Requirement: Strict mode fails on warnings

WHEN `--strict` is passed, a check run SHALL fail when `stats.findings` is
non-zero, regardless of severity. `stats.findings_blocking` SHALL continue
to count only error-severity findings.

#### Scenario: Warning-only run passes by default and fails strict

- **WHEN** a run reports only warning-severity findings
- **THEN** the default run exits 0, and the same run with `--strict`
  exits 1 with `ok: false`

## MODIFIED Requirements

### Requirement: Channel attribution

The system SHALL parse source files with pinned tree-sitter grammars and
SHALL attribute every comment or doc-string node to exactly one class in
this precedence: file-header, doc, marker, rationale, suppression,
exemption, finding-eligible prose. A leading `SAFETY:`, `INVARIANT:`,
`SECURITY:`, or `WHY:` token with a non-empty body SHALL be rationale,
counted in `stats.comments_rationale`; `NOTE:` SHALL remain prose.
Attribution SHALL be purely structural (node kind, position, leading
token), never semantic.

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

#### Scenario: Precedence keeps doc and header above rationale

- **WHEN** a file contains `//! SAFETY: the buffer is pinned` and a
  `/// WHY: callers rely on this order`
- **THEN** the first is header and the second is doc, not rationale

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
`smell` and `severity` assigned by the smell requirements. File-scoped
findings (`missing_header`, `suppression_cap_exceeded`) use `-` as the
symbol-path component.

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

#### Scenario: Marker-bearing prose is not a prose finding

- **WHEN** a TODO or apology comment carries a resolvable `see <id>`
  marker
- **THEN** channel attribution makes it a marker and no
  `comment_outside_channel` finding is emitted

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
sentences. Boundaries SHALL be terminal punctuation followed by
whitespace and an uppercase character or end of lead; inline code, URLs,
abbreviations, and version tokens SHALL NOT be boundaries. Rendered line
count SHALL NOT matter. Content after the first blank line SHALL NOT be
capped. The cap SHALL be configurable (default: 1 sentence).

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
  first blank line, however the source wraps them (the name is retained
  from the line-based rule; the failure condition is sentence count)
- **THEN** `doc_summary_cap_exceeded` is emitted with
  `severity: warning`

#### Scenario: Abbreviations and versions do not split sentences

- **WHEN** a summary lead reads `Parse e.g. v0.1 manifests from Node.js`
- **THEN** it counts as one sentence and no doc finding is emitted

#### Scenario: Non-public docstrings are capped equally

- **WHEN** a private Python function carries a multi-sentence-lead
  docstring
- **THEN** `doc_summary_cap_exceeded` is emitted (the doc channel covers
  all docstrings; there is no visibility carve-out in v1)

### Requirement: Output contract mirrors pretender

WHEN a check run completes, the system SHALL emit a genesis Check
envelope with findings carrying stable IDs, `severity`, optional `smell`,
and remediation, plus `suppression_debt` and channel stats. `ok` and the
exit status SHALL derive from new error-severity findings; warnings SHALL
NOT fail a default run. `stats.findings` SHALL count every new finding
and `stats.findings_blocking` the new errors. Baseline suppression and
shrink-only adoption SHALL be unchanged.

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

### Requirement: Suppression debt is exempt-but-counted

The system SHALL report every suppression directive (`# noqa`,
`# type: ignore`, `// @ts-ignore`, `// eslint-disable-*`, or
`sad:ignore <check-id> <reason>`) as a `suppression_debt` entry (file,
line, directive, check) in the envelope instead of a prose finding for
the directive node itself. A directive scopes to exactly one check and
SHALL NOT withhold any other finding; an error-severity finding remains
blocking. WHEN a `suppression_cap` is
configured and a single analyzed file's debt exceeds it, the system SHALL
emit `suppression_cap_exceeded`. The default SHALL be no cap.

#### Scenario: noqa counted, not flagged

- **WHEN** a Python line carries `# noqa`
- **THEN** the envelope contains a `suppression_debt` entry for it and
  no `comment_outside_channel` finding

#### Scenario: Cap breach

- **WHEN** `suppression_cap = 5` is configured and a file carries 6
  suppressions
- **THEN** `suppression_cap_exceeded` is emitted; with the default (no
  cap) it is not

#### Scenario: Directive does not silence an error smell

- **WHEN** a file carries `sad:ignore comment_outside_channel reason`
  next to a narration comment
- **THEN** the directive is counted in `suppression_debt`, the narration
  finding remains with `severity: error`, and the run fails