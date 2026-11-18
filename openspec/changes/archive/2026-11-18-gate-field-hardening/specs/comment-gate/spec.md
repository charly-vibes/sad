# comment-gate specification delta

## ADDED Requirements

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

## MODIFIED Requirements

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