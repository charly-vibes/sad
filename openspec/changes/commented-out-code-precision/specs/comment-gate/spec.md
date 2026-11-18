# comment-gate specification delta

## MODIFIED Requirements

### Requirement: Findings carry severity

Every released finding SHALL carry `severity`, either `error` or
`warning`. `error` SHALL apply to narration, apology, todo,
`commented_out_code`, `missing_header`, `unresolvable_marker`, and
`suppression_cap_exceeded`; `warning` SHALL apply to banner, unclassified,
and `doc_summary_cap_exceeded`. `smell` SHALL be null for findings not
derived from comment prose.

#### Scenario: Structural findings are errors with a null smell

- **WHEN** a run emits `missing_header`, `unresolvable_marker`, or
  `suppression_cap_exceeded`
- **THEN** each carries `smell: null` and `severity: error`

#### Scenario: Block findings are warnings until precision lands

- **WHEN** the block classifier reclassifies a separator-bodied block
  finding as a banner
- **THEN** that finding carries `smell: banner` and `severity: warning`;
  every matched non-banner block is an error, as the next scenario states

#### Scenario: Block findings are errors

- **WHEN** the block predicate matches a non-banner body
- **THEN** the `commented_out_code` finding carries
  `smell: commented_out_code` and `severity: error`

#### Scenario: Doc cap is a warning

- **WHEN** a doc summary exceeds the sentence cap
- **THEN** the `doc_summary_cap_exceeded` finding carries
  `smell: doc_cap` and `severity: warning`

### Requirement: Commented-out code is a block-scoped finding

One `commented_out_code` finding SHALL cover each maximal run of
adjacent finding-eligible prose nodes whose normalized body parses as a
syntactic unit. A normalized body with no alphanumeric character SHALL
NOT enter the class. The parse root SHALL carry at least one child that
is a declaration or a control statement; a root consisting only of bare
expression-statement payloads — calls, assignments, new-expressions,
formulas, or annotations — SHALL NOT match. It SHALL anchor at the run's
first node, hash the normalized block body as
`<path>::<symbol-path>::<content-hash>`, and carry the hardcoded
`git log -p --follow <path>` remediation (delete; recover from VCS). A
directive scoped to `commented_out_code` is ordinary suppression debt,
never prose. The check SHALL fail closed to prose and default on.

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

#### Scenario: Formula and annotation shapes stay prose

- **WHEN** a Rust comment reads `// E = m * c^2`, a Python comment reads
  `# E = m * c ** 2`, and an expected-value annotation such as
  `// x = 1;` sits beside the `assert_eq!(x, 1)` it documents
- **THEN** none emits a `commented_out_code` finding; each stays a
  per-node prose finding, because a root of bare expressions is not
  code-shaped

#### Scenario: Call-shaped examples stay prose

- **WHEN** a plain comment reads `// e.g. g();` or `// f();`, a
  TypeScript comment reads `// new Foo();`, and a labeled comment reads
  `// Note: retry();`
- **THEN** no `commented_out_code` finding is emitted; a bare call, a
  new-expression, and a labeled statement are not declarations or
  control statements, so each stays a per-node prose finding

#### Scenario: Separator bodies never reach the block class

- **WHEN** an adjacent prose run normalizes to a body with no
  alphanumeric character, such as a `// =====` ruler or a
  `/* ----- */` divider
- **THEN** no `commented_out_code` finding is emitted and each node
  stays a per-node prose finding; the body is rejected before parsing

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
- **THEN** no `commented_out_code` finding is emitted; the previously
  accepted false positive now stays prose under the call-shaped rule

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