# Proposal: comment-smell-triage

## WHY

The field investigation over 48 charly-vibes repos (3,873 files, 58,781
findings) shows the gate currently enforces comment placement, not code
quality:

- 42.5% of findings are in tests; 19.0% sit inside each file's leading
  comment block; 24.7% is mechanical noise (banners, shebangs, generated
  docs, agent worktrees).
- The stated target classes are nearly absent: TODO/FIXME 19 (0.03%),
  workaround/apology 26 (0.04%).
- Every inspected `commented_out_code` finding (14/14) was a banner, a
  math formula, or an expected-value annotation — zero dead code.
- 40% of `doc_summary_cap_exceeded` findings are single-sentence
  summaries that wrap across source lines; 6/8 external evidence sources
  contradict a line-based cap.

A prose comment is a smell only when it substitutes for expressiveness.
Treating every prose node as equal debt makes the gate delete knowledge
(contracts, SAFETY invariants, test intent) instead of improving code,
and a warning-free pass cannot distinguish a clean file from a loudly
documented one.

## What Changes

- **Findings name the smell.** Every finding gains `smell` and `severity`.
  Error smells: `narration` (conservative, code-restating), `apology`
  (for now/temporary/workaround/hack), `todo` (unresolvable TODO/FIXME),
  `commented_out_code`. Warning smells: `banner`, `doc_cap`,
  `unclassified` prose.
- **The verdict blocks on errors only.** `ok` and the exit status derive
  from new error-severity findings; warnings are reported and counted in
  `stats.findings_blocking`/`stats.findings` but do not fail the run.
- **Rationale becomes a channel.** Leading tags `SAFETY:`, `INVARIANT:`,
  `SECURITY:` are a structural rationale channel, counted in
  `stats.comments_rationale`, so required invariants stop being debt.
- **Doc cap counts sentences, not rendered lines.** One sentence passes
  however it wraps; a multi-sentence lead is a warning.
- **Remediation names dispositions, not deletion.** Per smell kind the
  hardcoded remediation offers rename/extract, move to header/doc, cite a
  marker, add a test, or (banners only) delete.
- **Enforcement is the baseline delta.** A repo adopts once; only new
  ids block. Without a baseline the run is an inventory of the tree.
- **Block findings whose body is a separator are banner warnings**, so
  the existing precision gap does not turn banners into errors.

## Impact

- Spec: `openspec/specs/comment-gate/spec.md` gains two requirements
  (smell/severity, baseline-delta enforcement) and modifies four
  (channel attribution, prose outside channels, doc summary cap, output
  contract).
- Code: `sad/src/model.rs`, `checks/outside_channel.rs`, `checks/doc_cap.rs`,
  `checks/mod.rs`, `checks/commented_out_code.rs`, `attribution.rs`,
  `check.rs`, `envelope.rs`, `render.rs`, `config.rs`.
- Output: additive `smell`, `severity`, `stats.findings_blocking`,
  `stats.comments_rationale`; `ok`/exit semantics change (0.2.0).
- Tests: every contract touching prose, doc cap, verdict, text mode.
- Docs: `docs/src/gate-model.md` (classification table, dispositions,
  verdict), `CHANGELOG.md`, config table (`doc_summary_cap` sentences).
- Ticket: `sad-66v` (feature, needs-spec-change).

## Dependencies (not bundled)

- `sad-h3i` shebang/PEP 263 false positives: currently 245 error-class
  prose findings; must land before the diff gate is enabled fleet-wide.
- `sad-8ah` gitignore/generated-path off-ramp: 1,985 findings in
  `_book/` and `.claude/worktrees`.
- `sad-6z5` non-SPDX licence headers.
- A later `ddl` change for complexity escalation (see Out of scope).

## Out of scope

- NLP or model-based comment classification. Heuristics are
  deterministic and conservative; ambiguous prose is `unclassified`
  (warning), never an error.
- Reading pretender metrics from sad. Complexity escalation is an
  orchestrator rule in `ddl`: a `narration` warning may become blocking
  when a structure gate flags the same unit. The boundary stays intact.
- Remediation-outcome telemetry (comment-only vs code-changing fixes);
  follow-up change, needed to prove the quality claim.
- Fixing the remaining noise classes inline; they keep their own
  tickets and spec changes.