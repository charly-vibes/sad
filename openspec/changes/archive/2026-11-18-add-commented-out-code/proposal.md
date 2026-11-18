# Proposal: add-commented-out-code

## WHY

Commented-out code is already a finding in the deployed gate: it is
finding-eligible prose and emits `comment_outside_channel` per node
(bd `sad-wfj`). Only its identity and remediation are wrong. Holding
`// fn parse() { ... }` — or a twelve-line dead block — to a prose
template ("move to a channel, die, or the code improves") is useless,
and it teaches agents to hide code inside a legitimate channel instead
of deleting it. Dead code differs from prose in exactly two things: what
the finding is called, and what the agent is told to do (delete it;
recover from VCS history when it is genuinely needed). It does not
differ in eligibility, attribution, or default-on status, so the class
is safe to ship enabled.

The design is settled in
`.wai/projects/sad/designs/2026-11-17-commented-out-code-finding-class.md`:
the predicate is "does the stripped block body parse as a syntactic
unit?" under the pinned grammar — deterministic, no model, no intent
inference. Evidence status is recorded honestly there (SonarQube S125
prior art; Pham & Yang ICPC 2020 prevalence; the deletion bet).

## What Changes

- Adds `FindingKind::CommentedOutCode` beside `comment_outside_channel`.
  Attribution is unchanged: every node stays one of the six
  `AttributionClass` variants; the new kind changes which check reports
  a node group, never the node's class.
- Emits per **block**, not per node: one `commented_out_code` finding per
  maximal run of adjacent finding-eligible prose nodes whose normalized
  body parses as a syntactic unit. Anchor = first node of the run; body =
  the normalized block body; ID = `path::symbol-path::content-hash` over
  that body. A twelve-line dead block is one finding.
- Adds the emission exception to the deployed prose requirement: when
  the check is enabled and a block matches, its member nodes emit no
  `comment_outside_channel` finding. A disabled check or a predicate
  miss leaves per-node prose exactly as deployed — never silence.
- Adds the `[checks] commented_out_code` toggle, default `true` (no
  threshold, so no `_exceeded` companion). Off downgrades a matched
  block to per-node prose findings, never to nothing.
- One hardcoded remediation per kind: `git log -p --follow <path>`
  with the description "delete the commented-out code; recover it from
  VCS history if still needed; never restore it as a comment". Never
  configurable, never executed; `<path>` renders at emission.
- Suppression scopes to the `commented_out_code` check id, so a block is
  silenced by one directive rather than N prose directives.

## Impact

- Specs: `openspec/specs/comment-gate/spec.md` gains one requirement
  and an emission exception in "Prose outside channels is a finding".
- Code: `sad-wfj` in `sad/src/checks/outside_channel.rs` (block
  grouping + emission exclusion), `sad/src/model.rs` (`FindingKind`),
  `sad/src/checks/` (the parse predicate and per-language tables),
  `sad/src/remediation.rs` (the new template), config and docs.
- Contracts: espectacular gains scenarios for the new requirement and
  the modified one under `.espectacular/changes/add-commented-out-code/`.
- Baseline: prose IDs are unchanged, but a dead-code finding hashes the
  block body and anchors at the first node — an ID no prose node had, so
  existing baselines are unaffected until re-adopted.
- Docs: `docs/src/gate-model.md` gains the check and its toggle.

## Out of scope

- No seventh `AttributionClass`: the new kind is a finding kind, not an
  attribution outcome.
- No length, ratio, or density thresholds; no `_exceeded` check.
- No new envelope stats (dead-code line counts are deferred to a later
  spec change).
- No change to header, doc-channel, marker, suppression-debt, baseline,
  or scoping behavior; doc nodes never enter this class.
- No grammar upgrades beyond the existing pretender pins; unknown node
  kinds fail closed to prose rather than widening the tables.
