# Proposal: specify-gate-objective

## WHY

bd `sad-lcy` (P1, "Specify the sad gate objective: what counts as prose
doing work the code should do") requires the gate model to land as an
EARS spec before implementation. The gate model was resolved in the
grill-me interview (wai design artifact, 2026-11-16), stress-tested by an
8-provider evidence review (wai research record, 2026-11-17), and
hardened by a Rule-of-5 review into "Consideration set v2" (same
record). `docs/src/gate-model.md` records the pre-review design and is
the doc this spec supersedes in three places (summary cap, header
default, scoping).

Premise being gated: comments and doc-strings are an apology for code
that failed to express itself; the fix is a refactor, not more prose.
Primary audience: LLM coding agents. Evidence status is recorded
honestly: narration-is-noise and the suppression envelope are
evidence-backed; the three-channel architecture is design-inherited;
the header requirement and ban causality are bets (see design.md).

## What Changes

- Adds capability `comment-gate`: deterministic, tree-sitter-based
  detection of prose outside three legitimate channels (file header,
  summary-capped doc comments, resolvable traceability markers), with
  exempt-but-observable suppression debt.
- Four checks: `comment_outside_channel`, `doc_summary_cap_exceeded`
  (v2: caps the summary lead, not the whole doc),
  `unresolvable_marker`, `missing_header` (v2: dogfood-only default).
- Doc comments (any visibility), summary-capped — the doc channel covers
  all docstrings and doc-comments; the cap stops agents laundering
  inline narration into doc-strings.
- Scoping: default mode analyzes only files carrying an explicit
  agent-attribution marker (or with no commit yet); `--all` lifts the
  scope (v2, adoption guard for the <10% false-positive threshold).
- Output contract mirrors pretender: genesis Check envelope, stable
  finding IDs `path::symbol-path::content-hash`, `--adopt` baseline
  (grandfather, shrink-only), hardcoded remediation templates.
- v1 language frontends: Rust, Python, TypeScript (pinned tree-sitter
  grammars, pretender's known-good pins).

## Impact

- Specs: `openspec/specs/comment-gate/spec.md` created on archive
  (currently no deployed specs).
- Code: none exists yet (greenfield); implementation follows in apply
  stage under bd sad-lcy → implementation tickets.
- Docs: `docs/src/gate-model.md` must be updated to the v2 model in the
  same change (it currently records the superseded ≤3-line cap).
- espectacular/ah wiring: contract tests get declared when the spec
  deploys; this proposal is documentation-only.

## Out of scope

Intent classification; comment-density metrics; blocking on suppression
debt by default (cap knob exists, default off); prompt-side or
agent-side integration; non-v1 languages; non-code comment surfaces
(SQL/shell/config/YAML); reader-side documentation quality; commented-
out-code as a distinct finding class (deferred; tracked for a later
change).
