# sad — gate design (grill-me interview, 2026-11-16)

## Core thesis

sad is a guardrail against LLM coding agents, not a human linter: agents
spray explanatory/apology comments to look done. Purist stance — any
comment is a violation — because agents respond to crisp mechanical rules,
and "no comments" is unambiguous where taxonomies are arguable.

## The pressure system (4 deterministic checks)

1. `comment_outside_channel` — prose outside the three legitimate channels
   is a finding. Tree-sitter comment-node extraction with channel
   attribution (grammar set v1: Rust, Python, TypeScript).
2. `doc_cap_exceeded` — pub-API doc-comments allowed, length-capped
   (≤3 lines, contract only). The cap stops agents laundering inline
   narration into doc-strings.
3. `unresolvable_marker` — traceability comments must cite an ID that
   resolves in `.beads/issues.jsonl` or `openspec/specs/` in this repo.
   Unresolvable ID = finding (anti-fabrication).
4. `missing_header` (`require_header`, default on in sad's repo) — every
   source file carries the Purpose/Responsibilities/Rationale header.
   Prose has exactly one destination; ban+require completes the squeeze.

## Exempt-but-observable principle

Every exemption channel appears in the envelope: header count, doc count,
markers resolved, `suppression_debt` (`# noqa`, `# type: ignore`,
`// @ts-ignore`, `// eslint-disable` etc. — exempt-but-counted, optional
caps, report-only in v0.1). An agent cannot slowly fill an exempt channel
without the numbers moving.

## Output contract (pretender mirror)

`sad check` → genesis envelope `EnvelopeKind::Check`: findings +
suppression_debt + channel stats. Stable finding IDs
`path::symbol-path::content-hash` (survive line drift). `--adopt` writes
`sad-baseline.json` (grandfathered IDs, committed; shrinking = incentive).
Single gate mode — boolean findings make gate+baseline equivalent to
pretender's gate/tiered split. Zero new integration concepts for
ddl/lefthook/CI.

## Remediation directives

Hardcoded per-check templates, not user-configurable in v1. Each finding
carries a genesis `RemediationEntry`: prose moves to a legitimate channel,
dies, or the code improves. Templates never say *which* refactor —
naming/extracting is the agent's job; sad refuses prose as substitute.

## Boundary statement

pretender gates structure (complexity/duplication/mutation — no prose
metrics verified in its metric list). sad gates prose. vampiro gates
composition. No tool reads another's input; complex-function + explanatory
comment is intentional stacking (both fire, same fix).

## Family conventions carried

`sad init` two-tier (standalone wizard + `ddl init --gates` fleet wiring),
`sad.toml` clones `pretender.toml` shape, grammars inherit pretender's
known-good pins. Languages: Rust ships first (self-host), then Python,
TypeScript — all three in v1.

## Open items (research next)

See the pinky/catitas research agenda in the interview wrap-up: prior art
on comment-antipattern evidence, doc-cap usability risk, suppression-debt
correlates, header-format enforcement friction.

