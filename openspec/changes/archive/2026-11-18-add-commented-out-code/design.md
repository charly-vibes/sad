# Design: add-commented-out-code

## Context

The authoritative design is
`.wai/projects/sad/designs/2026-11-17-commented-out-code-finding-class.md`
(2026-11-17). This change lands only the spec delta that note requires;
the implementation ticket is `sad-wfj`. The delta maps to the note as
follows:

| Note section | Delta |
|---|---|
| Precedence: classification, not exemption | ADDED requirement: finding kind, not a seventh `AttributionClass`; prose emission stays per node |
| Block unit and normalization | ADDED requirement: maximal run, first-node anchor, normalized body |
| Spec delta required (this change) | MODIFIED "Prose outside channels is a finding": the emission exception |
| Predicate (exact) | ADDED requirement + "fails closed to prose"; the tables are code, pinned to the grammar |
| Near-misses that must not fire | ADDED scenarios: parse error, bare literal/identifier, label, annotation-only, bodyless signature; doc-channel scenario |
| Config knob | ADDED requirement: default on; MODIFIED scenario "Disabled check emits per-node prose" |
| Stable ID consequences | ADDED requirement ID form; design note below |
| Remediation (hardcoded) | ADDED requirement: `git log -p --follow <path>` |
| Test plan (red phase) | tasks.md steps 1–9 |
| Known false positive | Pinned as a scenario, not fixed |

## Decisions

1. **Classification stays put.** Attribution runs first and is
   unchanged; every eligible node remains `Prose`. `commented_out_code`
   is a `FindingKind` beside `comment_outside_channel`, so it changes
   which check reports a node group, never the node's class. That keeps
   the six-variant `AttributionClass` closed and avoids re-deriving
   header/doc/marker behavior.
2. **Block, not line.** A multi-line statement cannot be recovered
   line-wise, so the block is the smallest possibly-complete unit. A
   block is a maximal run of finding-eligible nodes with no row gap
   (`next.line <= prev.line + prev.text.lines().count()`); a
   non-eligible node, code line, blank line, or symbol boundary ends it.
   Same-row nodes join. Anchor = first node, symbol path = the anchor's
   enclosing symbol (`-` file-scoped).
3. **Normalization is a fixed pipeline.** (a) `\r\n` → `\n`, lone `\r`
   → `\n`, drop a leading BOM; (b) strip markers per kind — Rust/TS line
   `//` plus one following `/` or `!`, Rust/TS block `/*` plus one
   following `*`/`!`, trailing `*/`, then per line leading whitespace,
   one `*` only if the rest is empty or starts with whitespace, and at
   most one space, Python one `#` and one space; (c) never strip `#`
   inside a Rust/TS body (`#[test]` is code); (d) dedent by the common
   whitespace prefix of non-blank lines, trim trailing whitespace and
   leading/trailing blanks, join with `\n`; empty body → prose, stop.
   Order matters: marker stripping before dedent, else gutters distort
   the common prefix.
4. **Predicate = parse, not regex.** Parse the normalized body with the
   same pinned frontend used for attribution. Dead iff: no `ERROR` node
   and no `is_missing()` node; the root's named children (comments
   ignored) are non-empty; every root named child kind is in a closed
   per-language `ROOT_ALLOW` table; some node has a `SHAPE` kind (call,
   `new`, assignment, declaration with a name/pattern, item/decl,
   import/use/export, macro invocation, control flow, or
   `return`/`throw`/`raise` carrying an argument — keyword-only
   `return`/`break`/`continue`/`pass`/`yield` do not count); and the
   `ROOT_DENY` table holds — no `expression_statement` payload is a bare
   identifier/literal, unary/binary/subscript/parenthesized expression
   or annotation-only Python assignment; no root named child is a
   bodyless signature or `labeled_statement`. `ROOT_DENY` outranks
   `SHAPE`. Unknown kinds reject. The three tables are pinned to the
   grammar version; this is the only place language knowledge lives.
5. **Emission exception is narrow.** When the check is enabled and a
   block matches, its member nodes emit no `comment_outside_channel`.
   A disabled check or a predicate miss leaves per-node prose exactly
   as deployed — the failure mode is always prose, never silence, so
   the toggle can never hide a finding.
6. **Config knob.** `[checks] commented_out_code = true` (default
   `true`). No threshold, so no `_exceeded` companion and no stats.
   Off downgrades a matched block to N per-node prose findings.
7. **Suppression is per check.** `sad:ignore commented_out_code
   <reason>` silences a block with one directive, matching the deployed
   one-check-per-directive scope; dead code caught this way stays
   observable as suppression debt.

## ID consequences

Format is unchanged: `path::symbol-path::content-hash`, FNV-1a 64-bit
over the body, 16 lowercase hex chars, kind not in the ID. Prose IDs are
the landed per-node IDs (line drift preserves, body edits change). A
dead-code finding hashes the *block* body and anchors at the first node
— an ID no prose node had. Therefore baselines survive a knob flip only
for single-node blocks; for a multi-node block, flipping the knob
replaces one ID with N. Decide adoption state before baselining, or
re-adopt. This is why the delta does not change prose emission, IDs, or
the baseline algorithm.

## Known false positive

Plain-comment documentation examples (`// e.g. g();`,
`// for example step.where(c);`) parse as calls and fire. Doc-channel
nodes (`///`, `//!`, `/**`, docstrings) are excluded before the
predicate, which covers the common case; extending the guard to plain
comments would require a textual prefix heuristic and break the
determinism rule ("only parse"). Accepted and pinned by the
"Documentation call example is a known false positive" scenario. If it
ever needs fixing, that is a spec change, not a table tweak.

## Alternatives considered

- **Seventh `AttributionClass`** — rejected: dead code is not a channel
  outcome, and attribution must stay stable so prose, header, doc, and
  marker behavior do not move.
- **Per-line dead-code findings** — rejected: a multi-line statement is
  not recoverable line-wise; per-line would manufacture findings for
  fragments of one removal.
- **Regex / textual heuristics** (SonarQube S125-style) — rejected:
  grammar-pinned parsing is already available and needs no wording
  table; unknown constructs can fail closed instead of guessing.
- **Length or ratio thresholds** — rejected: no evidence-based cut, and
  a threshold would need a cap/`_exceeded` surface the class does not
  need.
- **Excluding disabled tests** — rejected: `// #[test] fn ...` is
  non-executing code, intent is undecidable, and the remediation is
  identical.

## Risks

- **Grammar drift**: the tables are pinned to pretender's versions;
  a grammar bump must re-audit them (unknown kinds fail closed, so
  drift produces prose findings, not false silence).
- **Snippet language mismatch**: a Rust snippet inside a TS file falls
  back to prose, not silence — safe but noisy; acceptable.
- **Knob-flip churn**: multi-node block IDs are not stable across the
  toggle; documented in ID consequences.

## Validation plan

Red-first fixtures from the note's test plan (each positive once per
language, near-miss negatives, CRLF ID equality, mixed-file counts,
knob-off per-node counts, ID stability under line drift then change on
body edit) before the predicate lands; `ah check` contracts declared
for every scenario in this delta; the deltas themselves validated with
`openspec validate add-commented-out-code --strict`.
