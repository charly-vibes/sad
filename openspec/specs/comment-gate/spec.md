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
attributed as finding-eligible prose, except member nodes of a block
matched by the `commented_out_code` check (see "Commented-out code is a
block-scoped finding"). File-scoped findings (`missing_header`,
`suppression_cap_exceeded`) use `-` as the symbol-path component.

#### Scenario: Narration comment

- **WHEN** a Rust function body contains `// This function calculates
  the total`
- **THEN** a `comment_outside_channel` finding is emitted with a stable
  ID anchored to the enclosing symbol path

#### Scenario: TODO stub

- **WHEN** a Python file contains `# TODO: make this faster`
- **THEN** a `comment_outside_channel` finding is emitted (TODO prose is
  not a channel)

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

### Requirement: Language frontends

The system SHALL support Rust, Python, and TypeScript via pinned
tree-sitter grammars (pretender's known-good pins) in v1.

#### Scenario: Per-language fixture conformance

- **WHEN** the per-language fixture suite runs (one fixture exercising
  every attribution class per language)
- **THEN** each language produces the expected finding/envelope set

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

### Requirement: Commented-out code is a block-scoped finding

One `commented_out_code` finding SHALL cover each maximal run of
adjacent finding-eligible prose nodes whose normalized body parses as a
syntactic unit. It SHALL anchor at the run's first node, hash the
normalized block body as `<path>::<symbol-path>::<content-hash>`, and
carry the hardcoded `git log -p --follow <path>` remediation (delete;
recover from VCS). A directive scoped to `commented_out_code` is
ordinary suppression debt, never prose. The check SHALL fail closed to
prose and default on.

#### Scenario: Rust line-comment block is one finding

- **WHEN** a Rust file contains a four-line `//` run spelling a complete
  `fn` with a body
- **THEN** exactly one `commented_out_code` finding is emitted for the
  block, anchored at the run's first comment node

#### Scenario: Rust block comment is one finding

- **WHEN** a Rust file contains `/* let x = 1; */` and, separately, a
  gutter-decorated `/*` block spelling `fn f() {}`
- **THEN** each comment node yields one `commented_out_code` finding
  (block gutters are stripped before the parse)

#### Scenario: TypeScript and Python equivalents are block findings

- **WHEN** a TypeScript file contains `// const x = await load();` and
  a Python file contains an adjacent `#` run spelling a `for` loop with
  a body
- **THEN** each yields one `commented_out_code` finding rather than one
  prose finding per line

#### Scenario: Normalization yields identical IDs

- **WHEN** the same Rust dead block is presented with CRLF line
  endings, `*` gutters, and inconsistent indentation
- **THEN** every form normalizes to the same block body and therefore
  the same finding ID

#### Scenario: Mixed file emits block findings and per-node prose

- **WHEN** a file holds a 12-line dead block, one prose line, a 3-line
  dead block, and two adjacent prose lines
- **THEN** two `commented_out_code` findings and three
  `comment_outside_channel` findings are emitted

#### Scenario: Parse-error and bare-expression near-misses stay prose

- **WHEN** a Rust comment reads `// This function calculates the
  total.`, `// 5`, or `// foo`, and a Python comment reads
  `# list[str] | None`
- **THEN** no `commented_out_code` finding is emitted for any of them;
  each stays a per-node prose finding

#### Scenario: Label and annotation-only near-misses stay prose

- **WHEN** a TypeScript comment reads `// Note: retry();` (a labeled
  statement) and a Python comment reads `# TODO: refactor` (an
  annotation-only assignment)
- **THEN** both stay prose findings, because the root deny table
  outranks a call shape

#### Scenario: Bodyless signature stays prose

- **WHEN** a Rust comment reads `// fn f();`
- **THEN** it is a prose finding, not a block finding

#### Scenario: Suppression id is commented_out_code

- **WHEN** a matched dead block is followed by a
  `sad:ignore commented_out_code reason` directive comment
- **THEN** the directive is counted in `suppression_debt` with check
  `commented_out_code`, it is not a prose node and does not join the
  block, and the block's own finding is unaffected

#### Scenario: Doc-channel code examples never enter the class

- **WHEN** a Rust `///` doc comment contains a fenced code block and a
  `//!` header contains code-looking lines
- **THEN** no `commented_out_code` finding is emitted, because doc and
  header nodes are excluded before the predicate runs

#### Scenario: Block adjacency joins same-row and splits on separators

- **WHEN** two comment nodes share a source row and a further comment
  is separated from them by a blank line or a code line
- **THEN** the same-row nodes form one block and the separated comment
  starts another, each judged on its own body

#### Scenario: Documentation call example is a known false positive

- **WHEN** a plain comment reads `// e.g. g();`
- **THEN** one `commented_out_code` finding is emitted; only doc
  channels are excluded, so this accepted false positive is pinned
