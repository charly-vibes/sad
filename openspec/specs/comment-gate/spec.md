# comment-gate Specification

## Purpose
The comment gate: deterministic checks that keep comments, doc strings, and
traceability markers inside the three legitimate prose channels (file header,
public-API doc summary, resolvable marker) and turn every other prose node
into a finding with stable IDs, remediation, and a shrink-only baseline.

## Requirements

### Requirement: Channel attribution

The system SHALL parse source files with pinned tree-sitter grammars and
SHALL attribute every comment or doc-string node to exactly one class:
file-header channel, doc channel, marker channel, suppression directive,
exemption, or finding-eligible prose. Attribution SHALL be purely
structural (node kind, position, leading token) — never semantic.

#### Scenario: Rust file with one node per class

- **WHEN** a Rust file contains a `//!` header block, a `///` doc on a
  `pub` item, a `see sad-lcy` marker comment, a `#[allow]`-adjacent
  `// sad:ignore lorem`, an SPDX licence block, and an inline
  `// this function calculates the total` narration comment
- **THEN** attribution assigns each node to header / doc / marker /
  suppression / exemption / prose respectively

#### Scenario: TypeScript and Python parity

- **WHEN** equivalent fixtures are analyzed for TypeScript (`/** */` on
  an exported symbol, module header block comment) and Python (docstring
  on an exported symbol, module docstring)
- **THEN** attribution produces the same class assignment pattern as the
  Rust fixture

### Requirement: Prose outside channels is a finding

The system SHALL emit a `comment_outside_channel` finding with stable ID
`<path>::<symbol-path>::<content-hash>` for every comment node
attributed as finding-eligible prose. File-scoped findings
(`missing_header`, `suppression_cap_exceeded`) use `-` as the
symbol-path component.

#### Scenario: Narration comment

- **WHEN** a Rust function body contains `// This function calculates
  the total`
- **THEN** a `comment_outside_channel` finding is emitted with a stable
  ID anchored to the enclosing symbol path

#### Scenario: TODO stub

- **WHEN** a Python file contains `# TODO: make this faster`
- **THEN** a `comment_outside_channel` finding is emitted (TODO prose is
  not a channel)

### Requirement: Doc summary cap

The system SHALL emit a `doc_summary_cap_exceeded` finding when a
doc-channel comment's summary lead — the text up to the first blank
line — spans more than one rendered line. Content after the first blank
line (structured sections) SHALL NOT be capped. The cap value SHALL be
configurable in `sad.toml` (default: 1).

#### Scenario: One-line summary with structured sections passes

- **WHEN** a Rust `pub` item carries `/// Parses config.` followed by a
  blank doc line and `# Errors` / `# Panics` sections of any length
- **THEN** no doc finding is emitted

#### Scenario: Two-line summary fails

- **WHEN** a docstring's summary lead spans two lines with no blank line
  before the body
- **THEN** `doc_summary_cap_exceeded` is emitted for the doc node

#### Scenario: Non-public docstrings are capped equally

- **WHEN** a private Python function carries a multi-line-lead docstring
- **THEN** `doc_summary_cap_exceeded` is emitted (the doc channel covers
  all docstrings; there is no visibility carve-out in v1)

### Requirement: Traceability markers resolve

The system SHALL resolve marker-prefixed comment IDs against
`.beads/issues.jsonl` and `openspec/specs/`: a plain comment beginning
with the configured marker prefix (knob `marker_prefix`, default
`see`) and citing an ID matching the configured pattern (default
`\b[a-z][a-z0-9]*-[a-z0-9]+\b`) is marker-channel if
the ID resolves, and the system SHALL emit an `unresolvable_marker`
finding when it does not. Doc-channel nodes (doc-comments, docstrings)
SHALL always attribute to the doc channel regardless of their text —
they never route to the marker channel, so a resolvable ID prefix
cannot launder narration past the summary cap. Suppression directives
SHALL be recognized in any comment node kind and counted. Each
resolver SHALL no-op while its source (beads file, specs directory)
is absent.

#### Scenario: Resolvable ticket ID

- **WHEN** a comment is `// see sad-lcy: cap rule comes from review`
  and `sad-lcy` exists in `.beads/issues.jsonl`
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

### Requirement: File header requirement is dogfood-default

WHEN `require_header` is enabled, the system SHALL emit a
`missing_header` finding for each analyzed source file whose first block
does not contain all three header keywords (`Purpose`,
`Responsibilities`, `Rationale`, order-free). The default SHALL be
enabled only in sad's own repository configuration; other repositories
opt in explicitly.

#### Scenario: Header present, keywords out of order

- **WHEN** `require_header` is on and a file's first block reads
  "Rationale: … Responsibilities: … Purpose: …"
- **THEN** no `missing_header` finding is emitted

#### Scenario: Partial header fails

- **WHEN** `require_header` is on and a file's first block contains
  only `Purpose:`
- **THEN** a `missing_header` finding is emitted (all three keywords
  are required)

#### Scenario: Check disabled

- **WHEN** `require_header` is off (any repository other than sad
  itself, by default)
- **THEN** no `missing_header` findings are emitted

### Requirement: Suppression debt is exempt-but-counted

The system SHALL report every suppression directive (`# noqa`,
`# type: ignore`, `// @ts-ignore`, `// eslint-disable-*`, or
`sad:ignore <check-id> <reason>`) as a `suppression_debt` entry (file,
line, directive, check) in the envelope instead of a prose finding. A
directive scopes to exactly one check. WHEN a
`suppression_cap` is configured and a single analyzed file's debt
exceeds it, the system SHALL
emit `suppression_cap_exceeded`. The default SHALL be no cap (v0.1).

#### Scenario: noqa counted, not flagged

- **WHEN** a Python line carries `# noqa`
- **THEN** the envelope contains a `suppression_debt` entry for it and
  no `comment_outside_channel` finding

#### Scenario: Cap breach

- **WHEN** `suppression_cap = 5` is configured and a file carries 6
  suppressions
- **THEN** `suppression_cap_exceeded` is emitted; with the default (no
  cap) it is not

### Requirement: Exemptions are observable

The system SHALL exempt licence/SPDX header blocks, `@generated` marker
regions, and paths matching `sad.toml` exclude globs from findings, and
SHALL report exemption counts in the envelope.

#### Scenario: Generated file excluded

- **WHEN** a file starts with `// @generated` and matches no include
  override
- **THEN** its comments produce no findings; the envelope reports the
  file as exempt

#### Scenario: Exclusion glob

- **WHEN** `sad.toml` excludes `vendor/**` and a vendored file contains
  narration comments
- **THEN** no findings are emitted for that path

### Requirement: Agent-attribution scoping

In default mode, the system SHALL analyze only files whose governing
revision — the most recent commit touching the file — carries an
explicit agent-attribution marker (`Assisted-by:` commit trailer or
session tag). Files with no commit yet (new or uncommitted) SHALL be
analyzed in default mode (conservative: new files during agent work are
treated as agent-authored). WHEN `--all` is passed, the system SHALL
analyze every non-excluded file. Attribution SHALL come only from
explicit markers, never inferred from git heuristics.

#### Scenario: Default mode skips untouched legacy files

- **WHEN** default mode runs on a repo where only `src/new.rs` carries
  an `Assisted-by:` trailer in its latest commit
- **THEN** only `src/new.rs` is analyzed

#### Scenario: Full run

- **WHEN** `sad check --all` runs on the same repo
- **THEN** all non-excluded files are analyzed

### Requirement: Language frontends

The system SHALL support Rust, Python, and TypeScript via pinned
tree-sitter grammars (pretender's known-good pins) in v1.

#### Scenario: Per-language fixture conformance

- **WHEN** the per-language fixture suite runs (one fixture exercising
  every attribution class per language)
- **THEN** each language produces the expected finding/envelope set

### Requirement: Output contract mirrors pretender

WHEN a check run completes, the system SHALL emit a genesis Check
envelope containing findings (stable IDs), `suppression_debt` entries,
channel/exemption stats, and a hardcoded `RemediationEntry` per finding
kind. WHEN `--adopt` is passed, the system SHALL write
`sad-baseline.json`. WHEN a baseline exists, the system SHALL suppress
grandfathered findings and SHALL fail the run if the baseline would
grow. `sad check` SHALL run identically as a pre-hook and as a CI step
(exit code 0 = pass, 1 = findings or gate failure).

#### Scenario: Adopt and shrink-only

- **WHEN** `sad check --adopt` runs on a tree with findings, then a
  later run fixes prose (fewer findings)
- **THEN** the baseline suppresses grandfathered IDs and the later run
  passes; re-adopting a superset baseline fails the run

#### Scenario: Same command, both modes

- **WHEN** the identical `sad check` invocation runs as a pre-hook and
  in CI on the same tree
- **THEN** both produce the same envelope and exit code

#### Scenario: Remediation entry

- **WHEN** a `comment_outside_channel` finding is emitted
- **THEN** it carries a hardcoded remediation entry (move to a channel,
  delete, or refactor — never naming a specific refactor)
