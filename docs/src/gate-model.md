> *"It's sad (So sad), so sad*
> *It's a sad, sad situation*
> *And it's gettin' more and more absurd*
> *It's sad (So sad), so sad*
> *Why can't we talk it over?"*
> — Elton John

# Gate Model

> **Status: specified 2026-11-16 — not implemented.** This page records the
> resolved design (grill-me interview; rationale in the repo's wai design
> artifact, ticket `sad-lcy`). Nothing here runs yet; the EARS spec is the
> next deliverable. The [command surface](getting-started.md) lists what
> exists today.

## Premise

Code should be self-explanatory. Comments and doc strings are an apology for
code that failed to express itself — the fix is a refactor into simpler,
more expressive code, not more words.

sad's primary audience is **LLM coding agents**, which spray narration
("`# This function calculates the total..."`), apology comments
("`// HACK: this is confusing`"), and TODO stubs to look finished. Agents
respond to crisp mechanical rules, so sad is purist by design: anything
outside the legitimate channels below is a finding. No semantic judgment
anywhere — channels and counts only.

## Legitimate prose channels

Prose is allowed in exactly three places, all structurally checkable:

1. **File header** — a top-of-file module doc (`//!` in Rust, module
   docstring in Python, header block comment in TypeScript) carrying
   `Purpose / Responsibilities / Rationale`. The one home for "why" prose.
2. **Public-API doc-comments** — `///` on `pub` items, docstrings on
   exported Python symbols, `/** */` JSDoc on exported TypeScript symbols.
   Allowed but **length-capped** (≤ 3 lines: contract only — preconditions,
   postconditions, errors). The cap is what stops agents from laundering
   inline narration into doc-strings.
3. **Traceability markers** — comments whose leading ID resolves against a
   real ticket or spec section *in this repo* (`.beads/issues.jsonl`,
   `openspec/specs/`). An unresolvable ID is itself a finding — the
   anti-fabrication guardrail. Pure file lookup, no semantics.

Tool suppression directives (`# noqa`, `# type: ignore`, `// @ts-ignore`,
`// eslint-disable-*`, …) are **exempt-but-counted**: not findings, but
every occurrence is reported as `suppression_debt` (file, line, directive),
with optional caps. Report-only in v0.1.

**Exempt-but-observable principle:** every exempt channel appears in the
envelope — header count, doc count, markers resolved, suppression debt. An
agent cannot slowly fill an exempt channel without the numbers moving.

## Checks

| Check | Finding |
|---|---|
| `comment_outside_channel` | prose outside the three channels |
| `doc_cap_exceeded` | public-API doc-comment over the line cap |
| `unresolvable_marker` | traceability ID that resolves to nothing |
| `missing_header` | source file without the required header |

`require_header` is a config flag — on by default in sad's own repo and the
recommended `sad init` profile; off while adopting fleets retrofit headers
(ban-only tier).

## Remediation directives

Each finding carries a hardcoded per-check template (not user-configurable
in v1) as a genesis `RemediationEntry`. The templates encode the pressure
system: prose either *moves* to a legitimate channel, *dies*, or *the code
improves* (rename/extract until the comment is unnecessary). Templates never
say *which* refactor — naming and decomposition are the agent's job; sad
refuses prose as a substitute.

## Output contract (pretender mirror)

- `sad check` → genesis suite envelope (`Check` kind): findings list +
  `suppression_debt` + channel stats. Exit status mirrors `ok`.
- **Stable finding IDs** — `path::symbol-path::content-hash`: survive line
  drift and unrelated edits; die only when the comment itself moves or
  changes.
- **`--adopt`** → writes `sad-baseline.json` of grandfathered IDs (committed;
  shrinking it is the refactor incentive). Gate mode then fails only on new
  findings. Single mode — boolean findings make gate+baseline equivalent to
  pretender's gate/tiered split.
- Zero new integration concepts: lefthook/CI wiring mirrors
  [pretender](https://github.com/charly-vibes/pretender); `sad init` writes
  `sad.toml`, the hook entry, and a CI snippet; `ddl init --gates` wires the
  fleet-managed variant.

## Languages

Rust, Python, TypeScript — v1 scope, tree-sitter comment-node extraction
with channel attribution, grammar pins inherited from pretender's
known-good set. Rust ships first so sad gates itself.

## Boundary

[pretender](https://github.com/charly-vibes/pretender) gates structure
(complexity, duplication, mutation), sad gates prose,
[vampiro](https://github.com/charly-vibes/vampiro) gates composition. No
tool reads another's input; a complex function plus an explanatory comment
is intentional stacking — both fire, same fix.

## Determinism

No network, no models, no randomness. Same input tree plus the same repo
ledgers (`.beads/issues.jsonl`, `openspec/specs/`) → same findings. Marker
resolution is file lookup only.

## Open design knobs

Resolved late, before the EARS spec: exact doc-cap value per language (3
lines is the current guess — panics/errors/examples may need structured
allowances), header-format strictness (labels verbatim vs any block),
generated/vendor exclude defaults, grammar version pins.
