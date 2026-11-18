# comment-gate specification delta

## ADDED Requirements

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

## MODIFIED Requirements

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
