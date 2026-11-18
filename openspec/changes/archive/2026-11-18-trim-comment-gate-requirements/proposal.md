# Proposal: trim-comment-gate-requirements

## WHY

`openspec validate comment-gate --strict` warns that three deployed
requirements exceed the 500-character prose limit: "Traceability
markers resolve" (800), "Agent-attribution scoping" (531), and "Output
contract mirrors pretender" (525). The excess is not behavior — it is
parenthetical examples, knob defaults, and edge cases sitting in
requirement prose where the archive-time validator keeps flagging
them. Moving them into the scenarios that already exercise the same
behavior clears the warning without changing what the gate does.

## What Changes

- Rewrites the prose of exactly those three requirements to state one
  behavior each, under the 500-character limit. Requirement titles stay
  byte-identical.
- Moves the parenthetical examples and edge cases out of prose and into
  scenarios: marker knob defaults and openspec resolution, absent-source
  no-op, suppression recognition across comment kinds, uncommitted-file
  attribution and session tags, baseline adoption and exit codes.
- Keeps every existing scenario title byte-identical, so the 24
  espectacular contract mappings keyed by scenario slug stay valid.
  Adds new scenario titles only where the moved text has no existing
  scenario to land in.
- Behavioral no-op: no requirement is added, removed, renamed, or
  re-scoped, and no code changes.

## Impact

- Specs: `openspec/specs/comment-gate/spec.md` shrinks on archive; the
  requirement set and every existing scenario title are unchanged.
- Code: none.
- espectacular: contract mappings move unchanged; `ah check` must stay
  green.