# Design: specify-gate-objective

## Context

sad gates prose; pretender gates structure; vampiro gates composition.
No tool reads another's input; complex-function + explanatory comment is
intentional stacking (both fire, same fix). sad's audience is LLM coding
agents, so every rule must be mechanically decidable — no semantic
judgment anywhere. Output/integration concepts are cloned from pretender
(envelope, baseline, gate mode) so ddl/lefthook/CI wiring is zero-new.

This design records the decisions that survive the evidence review, and
the honest evidence status of each.

## Evidence-strength ledger (from wai research record, Consideration set v2)

| Decision | Status | Grounds |
|---|---|---|
| `suppression_debt` exempt-but-counted | evidence-backed | Topalidi 2026 validates the envelope "almost verbatim"; Hu FSE'25 (~50% useless) |
| `unresolvable_marker` | evidence-backed | staleness/traceability detection is measurable (CoCC 93.6% agreement; Linux 869 stale refs) |
| Three-channel structure | design-inherited | no architecture-level evidence (research prompt froze the design); components individually supported |
| Summary-lead cap (replaces ≤3-line doc cap) | evidence-backed redesign | 6/8 providers: ecosystem norms (Rust API Guidelines, PEP 257, TSDoc) contradict a total cap |
| Header requirement | bet | zero corpus support; prior art: presence mandates → stub noise (Go lints); dogfood-only default |
| Premise causality ("bans improve code") | bet | correlational only; KAKEN: position beats volume |

## Decisions

1. **Summary cap, not doc cap.** Doc-channel comments: the summary lead
   (text up to the first blank line) is capped at one line; everything
   after the first blank line (structured sections — Rust `# Errors`,
   numpy `:param:`, TSDoc `@throws`) is uncapped. Deterministic across
   languages; no section semantics, no intent judgment.
2. **Header demoted.** `require_header` defaults ON only in sad's own
   repo (dogfood). Detection is keyword-based and order-free
   (Purpose/Responsibilities/Rationale anywhere in the first block) —
   not verbatim labels, so stub-gaming at least requires reading the
   file's purpose. Justify or cut at spec review.
3. **No position carve-outs.** KAKEN's pre-method/in-method finding
   justifies the header channel's existence; it does NOT create an
   in-body exemption (that would reintroduce intent judgment).
4. **Agent-attribution scoping.** Default mode analyzes only files with
   an explicit `Assisted-by:` commit trailer (or session tag) in the
   governing revision. Never inferred from git heuristics (squash,
   rebase, multi-author files break inference; grandfathered baselines
   must not mask new agent edits). `--all` lifts scope.
5. **Marker pattern.** Traceability comments use an explicit marker
   prefix (config knob `marker_prefix`, default `see`) plus a
   repo-configurable ID pattern defaulting to
   `\b[a-z][a-z0-9]*-[a-z0-9]+\b` (matches real beads IDs like
   `sad-lcy`, `sad-8b7`; digit-required patterns were wrong). Resolvers:
   `.beads/issues.jsonl` + `openspec/specs/` — each resolver no-ops
   while its source is absent (a repo without beads never flags all
   markers unresolvable). Version-number-shaped text is never
   treated as a marker unless marker-prefixed.
6. **Suppression: scoped, counted, capped-by-knob.**
   Tool directives and sad's own `sad:ignore <check-id> <reason>` count
   as `suppression_debt`. The directive is check-scoped (one directive
   suppresses one check) — matching the evidence that narrowly scoped
   directives run ~0.5% false-positive association vs ~93.1% for bare
   keywords (Topalidi 2026; see wai research record). `suppression_cap`
   knob (default: off, per analyzed file) emits
   `suppression_cap_exceeded` on breach — a rational agent must not be
   able to route around the gate via ignore-spam with zero signal.
7. **Gate + baseline, no warn tier.** pretender-mirror: `sad check`
   exits 0/1; `--adopt` writes `sad-baseline.json`; grandfathered IDs
   are suppressed; baseline shrink-only (growing it is a gate failure).
   Warn-mode duplication of pretender's tiered concept is rejected.
8. **Remediation templates are hardcoded** per check, never
   user-configurable: prose moves to a channel, dies, or the code
   improves. Templates never prescribe *which* refactor.

## Config knob inventory (sad.toml)

`checks` toggles · `doc_summary_cap` (default 1) · `marker_prefix`
(default `see`) · `marker_pattern` (beads-style default) · `resolvers`
(beads, openspec — active only when source present) · `require_header`
(default true only in sad's repo) · `suppression_cap` (default off,
per analyzed file) · `exclude` globs · attribution source
(`Assisted-by` trailer).

## Alternatives considered

- **≤3-line total doc cap** (v1): contradicted by all three target
  ecosystems' norms; replaced by decision 1.
- **Position-aware in-body exemption** (KAKEN): rejected — intent leak.
- **Warn default with --max-warnings escalation**: rejected — duplicates
  pretender tiering; baseline already provides non-blocking adoption.
- **Commented-out-code finding class**: deferred — "body parses as
  language syntax" is decidable but the remediation story (delete vs
  refactor) needs its own design; tracked for a later change.

## Risks

- **Adoption false positives** (>10% effective FP kills linters,
  Sadowski CACM'18): mitigated by attribution scoping + exemptions.
- **Absorption**: defense is narrow scope — sad never grows metrics or
  structure checks (that's pretender).
- **Reader-side regression**: correct comments transfer knowledge to
  weaker models (+17.2% pass-rate study). Mitigation: the doc channel
  preserves structured sections and markers (the high-transfer signal);
  sad measures prose absence, not docs that help agents read code.
- **Citation hygiene**: spec rationale cites primary-checkable sources
  only; provider consensus is not a citation.

## Validation plan (falsifiability)

sad claims "gate compliance improves code" — currently unproven. The
change therefore ships with a measurement duty:

1. Dogfood: sad's own repo adopts the gate (require_header ON) and
   passes; findings on sad itself feed the remediation templates.
2. Finding-rate telemetry: run `sad check --all` on a panel of known
   agent-authored PRs vs human-authored baselines; publish rates in the
   repo (target: agent files carry the finding mass).
3. A/B path: when volume permits, compare defect/complexity drift
   (pretender metrics) on gated vs ungated files. If gating shows no
   signal after a defined window, the premise loses and the spec is
   revisited — sad must be falsifiable.
