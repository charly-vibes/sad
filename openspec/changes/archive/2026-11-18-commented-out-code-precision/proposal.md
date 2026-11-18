# Proposal: commented-out-code-precision

## WHY

The field investigation over 48 charly-vibes repos
(`changes/archive/2026-11-18-comment-smell-triage/evidence.md`) inspected
14 `commented_out_code` findings and found 14 bodies that were separator
banners, math formulas, or expected-value annotations — zero dead code.
The predicate accepted any shape node anywhere in the tree, so a bare call
(`f();`), a bare assignment (`x = 1;`), a new-expression (`new Foo();`),
and a documentation example (`e.g. g();`) all matched. The check stayed
warning-only until this precision work landed.

## What Changes

- **Separator bodies are rejected before parsing.** A normalized body with
  no alphanumeric character is a ruler or divider, never code, and never
  reaches the predicate.
- **The parse root must be code-shaped.** At least one root child SHALL be
  a declaration or a control statement, from a closed per-language
  `code_shape` table. A root made only of bare expression-statement
  payloads — calls, assignments, new-expressions, formulas, annotations —
  stays prose. Fail-closed handling of unknown kinds and `ERROR` nodes,
  root-allow, root-deny, payload-deny, and the chunking bounds are
  unchanged.
- **`commented_out_code` is an error again.** A matched non-banner block
  carries `severity: error`; `severity_of(Smell::CommentedOutCode)` agrees.
  The separator banner reclassification keeps `severity: warning`. Finding
  IDs, lines, symbol paths, and text do not change.
- **Spec delta.** `Findings carry severity` and `Commented-out code is a
  block-scoped finding` are modified; the block scenario becomes "Block
  findings are errors", and the precision corpus replaces the pinned
  documentation false positive.

## Impact

- Spec: `openspec/specs/comment-gate/spec.md` delta in this change covers
  severity and the block predicate contract.
- Code: `sad/src/checks/commented_out_code.rs`,
  `sad/src/checks/commented_out_code/predicate/mod.rs`,
  `sad/src/checks/commented_out_code/predicate/tables.rs`,
  `sad/src/checks/smell.rs`.
- Tests: `sad/tests/check_commented_out_code_test.rs`,
  `sad/tests/check_commented_out_code_predicate_test.rs`.
- Contracts: `.espectacular/changes/commented-out-code-precision/comment-gate/`.
- Ticket: `sad-jdw`.