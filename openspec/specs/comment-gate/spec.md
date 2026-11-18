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

The system SHALL support Rust, Python, TypeScript, and JavaScript via
pinned tree-sitter grammars (pretender's known-good pins) in v1. The
supported extension set SHALL be exactly `.rs`, `.py`, `.ts`, `.js`, and
`.mjs`; JavaScript files SHALL be parsed with the TypeScript frontend.
Adjacent extensions the gate cannot parse (`.tsx`, `.mts`, `.cts`, `.jsx`)
SHALL be discovered, skipped, counted in `files_unsupported`, and reported
as envelope warnings naming each path, never dropped silently. The set and
its exclusions SHALL be documented in `docs/src/gate-model.md`.

#### Scenario: Per-language fixture conformance

- **WHEN** the per-language fixture suite runs (one fixture exercising
  every attribution class per language)
- **THEN** each language produces the expected finding/envelope set

#### Scenario: JavaScript sources are gated

- **WHEN** a tree contains a `.js` and a `.mjs` file with prose comments
- **THEN** both are discovered, parsed with the TypeScript frontend, and
  their prose yields `comment_outside_channel` findings

#### Scenario: Unsupported adjacent extensions are visible

- **WHEN** a tree contains `.tsx`, `.mts`, `.cts`, and `.jsx` sources
- **THEN** each is counted in `files_unsupported` and named by one
  `unsupported_extension` envelope warning, and none produces findings

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

The predicate input SHALL be bounded: a normalized body within 8192 bytes
and 512 lines SHALL be parsed as one unit, and a longer body SHALL be
parsed in chunks cut at line boundaries with the same bounds, the run
matching when any chunk parses as a syntactic unit. The whole body SHALL
remain the finding's text, so chunking never changes an ID. Findings for
bodies within the bound SHALL NOT change. The bound and its fail-closed
chunk-boundary trade-off SHALL be documented in the check's source and in
`docs/src/gate-model.md`.

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

#### Scenario: Long prose runs are bounded

- **WHEN** a file holds a contiguous run of 32000 adjacent prose line
  comments with no code
- **THEN** the run completes under the regression bound instead of the
  pre-bound cliff, and every node is still reported (one prose finding
  per node, or one block finding when a chunk matches)

#### Scenario: Runs within the bound are unchanged

- **WHEN** a run's normalized body stays within 8192 bytes and 512 lines
- **THEN** it is parsed as a single unit and produces exactly the findings
  the un-chunked predicate produced

#### Scenario: A code chunk past the bound still matches the block

- **WHEN** a run longer than the bound contains a dead-code item wholly
  inside one chunk
- **THEN** one `commented_out_code` finding is emitted for the whole run
  with the full-body hash

### Requirement: Finding identity is unique per finding

Every emitted finding SHALL carry an ID unique within its run. The base
identity SHALL remain `<path>::<symbol-path>::<content-hash>`. When two or
more findings in one run share a base identity, the first occurrence in
report order (path, line, check id) SHALL keep the bare base identity and
each later occurrence SHALL append `#<n>`, where `n` is its 1-based
occurrence index. The discriminator is applied after all checks run, so a
collision between different check kinds is discriminated too. IDs SHALL NOT
depend on line numbers, so inserting or deleting unrelated lines changes no
ID.

`sad-baseline.json` SHALL carry `schema_version: 2`. A version-1 baseline
SHALL still suppress exact id matches, and the next `--adopt` SHALL migrate
it to version 2, permitting that single adoption to add ids; every later
adoption SHALL remain shrink-only.

#### Scenario: Duplicate bodies get distinct ids and both are adopted

- **WHEN** a file contains two prose comments with identical bodies inside
  one symbol and `sad check --all --adopt` runs
- **THEN** the report carries `<base>` and `<base>#2`, and the baseline
  stores both ids as separate entries

#### Scenario: Unique findings keep their bare identity

- **WHEN** a run reports one finding per distinct base identity
- **THEN** every ID is the bare `<path>::<symbol-path>::<content-hash>`
  with no discriminator

#### Scenario: Line drift does not churn ids

- **WHEN** ten unrelated lines are inserted above a finding and the run is
  repeated
- **THEN** the finding's ID is unchanged

#### Scenario: Version-one baseline migrates once

- **WHEN** `sad-baseline.json` declares `schema_version: 1` and
  `sad check --all --adopt` runs after the id scheme gained
  discriminators
- **THEN** adoption succeeds, writes `schema_version: 2`, and a subsequent
  adoption that would add an id is refused as baseline growth

#### Scenario: Unknown schema versions still fail loudly

- **WHEN** a baseline declares `schema_version: 3`
- **THEN** the run fails with a `baseline_error` naming the path and the
  version, and no baseline is rewritten

### Requirement: Scope decisions are observable

A gate run SHALL report, in its stats, how commit-attribution scope treated
the candidate files: `files_attributed` (kept because the governing revision
carries the configured trailer), `files_without_commit` (kept because git
has no governing revision), and `files_unattributed` (dropped because the
governing revision lacks the trailer). A full `--all` run resolves no
attribution and SHALL report all three as zero.

WHEN a default-mode run selects no files but candidates survived exclusion,
the text render SHALL state that the attribution scope excluded them and
SHALL name the widening paths (`sad check --all`, or committing with the
configured trailer). Selection semantics SHALL NOT change: working-tree
state is never evidence.

#### Scenario: Zero-selection run is distinguishable from a clean run

- **WHEN** default mode runs on a repository whose only candidate file was
  edited but not committed and whose governing revision carries no trailer
- **THEN** the stats report `files_selected: 0` and a non-zero
  `files_unattributed`, and text mode names the attribution scope and the
  `--all` widening path instead of only reporting a pass

#### Scenario: Attributed and uncommitted files are counted separately

- **WHEN** default mode sees one trailer-attributed file, one never-committed
  file, and one legacy file
- **THEN** `files_attributed: 1`, `files_without_commit: 1`,
  `files_unattributed: 1`, and `files_selected: 2`

#### Scenario: Full run resolves no attribution

- **WHEN** `sad check --all` runs over the same tree
- **THEN** `files_attributed`, `files_without_commit`, and
  `files_unattributed` are all zero and `files_selected` counts every
  non-excluded candidate

### Requirement: Unreadable sources are named by the envelope

Every selected source file the gate cannot read as UTF-8 SHALL be counted
in `stats.files_skipped` and named by exactly one `unreadable_source`
envelope warning carrying its repo-relative path and a reason: `not valid
UTF-8`, `permission denied`, `file disappeared during the run`, or the
operating-system error text for any other I/O failure. Analysis SHALL NOT
invent findings, guess an encoding, or change the exit status because of a
warning.

#### Scenario: Undecodable sources are named

- **WHEN** a tree contains one valid UTF-8 `.rs`, one latin-1 `.rs`, and
  one invalid-UTF-8 `.rs`, and `sad check --all` runs
- **THEN** `stats.files_skipped` counts both undecodable files, each has
  one `unreadable_source` warning carrying its path and `not valid UTF-8`,
  and no finding is derived from them

#### Scenario: Unreadable permissions are named

- **WHEN** a selected source file cannot be opened for reading
- **THEN** it is counted in `stats.files_skipped` and named by one
  `unreadable_source` warning with reason `permission denied`

#### Scenario: A fully readable run is unchanged

- **WHEN** every selected file is readable UTF-8
- **THEN** no `unreadable_source` warning is emitted and the text render is
  the same one-liner as before this change

### Requirement: Text mode never hides a skipped source

WHEN a check run skipped one or more selected files, the text render SHALL
append the skipped-file count and every skipped path with its reason to
its single line. Text mode SHALL read the same envelope warnings the JSON
consumer reads, so the paths and reasons cannot drift from the machine
contract.

#### Scenario: Text mode names skipped sources instead of a bare pass

- **WHEN** a run skips a latin-1 and an invalid-UTF-8 source
- **THEN** the text render names both paths with `not valid UTF-8` and the
  skipped count, instead of only reporting `pass`
