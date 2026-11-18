# Proposal: comment-smell-triage

## WHY

The field investigation over 48 charly-vibes repos (3,873 files, 58,781
findings; `evidence.md`) shows the gate currently enforces comment
placement, not code quality:

- 42.5% of findings are in tests; 19.0% sit inside each file's leading
  comment block; 24.7% is mechanical noise (banners, shebangs, generated
  docs, agent worktrees).
- The stated target classes are nearly absent: TODO/FIXME 19 (0.03%),
  workaround/apology 26 (0.04%).
- Every inspected `commented_out_code` finding (14/14) was a banner, a
  math formula, or an expected-value annotation — zero dead code.
- 39.6% of `doc_summary_cap_exceeded` findings are single-sentence
  summaries that wrap across source lines; 6/8 external evidence sources
  contradict a line-based cap.

A prose comment is a smell only when it substitutes for expressiveness.
Treating every prose node as equal debt makes the gate delete knowledge
(contracts, SAFETY invariants, test intent) instead of improving code,
and a warning-free pass cannot distinguish a clean file from a loudly
documented one.

## What Changes

- **Severity for every finding.** Findings carry `severity: error|warning`.
  `smell` names the prose smell and is `null` for structural findings
  (`missing_header`, `unresolvable_marker`, `suppression_cap_exceeded`).
  Error smells: `narration`, `apology`, `todo`, plus those structural
  findings. Warning: `banner`, `unclassified`, `doc_cap`, and
  `commented_out_code` until its precision ticket (`sad-jdw`) promotes it.
- **The verdict blocks on errors only.** `ok` and the exit status derive
  from new error-severity findings; warnings are counted but do not fail.
  `--strict` restores any-new-finding failure for teams that want it.
- **Rationale becomes a channel.** Leading `SAFETY:`, `INVARIANT:`,
  `SECURITY:`, and `WHY:` tags with a body are counted rationale, not
  debt. `NOTE:` stays prose.
- **Doc cap counts sentences, not rendered lines.** One sentence passes
  however it wraps; a multi-sentence lead is a warning.
- **Remediation names dispositions, not deletion.** Per smell kind the
  remediation offers rename/extract, move to a channel, tag as rationale,
  cite a marker, add a test, or (banners only) delete.
- **Enforcement is the baseline delta.** A repo adopts once; only new
  ids block. Without a baseline the run is a full inventory.
- **Block findings are warning-severity** until precision is fixed;
  separator bodies are `banner` regardless of the check id.

## Impact

- Spec: `openspec/specs/comment-gate/spec.md` gains four requirements
  (severity, smell classification, baseline-delta enforcement, strict
  mode) and modifies five (channel attribution, prose outside channels,
  doc summary cap, output contract, suppression debt).
- Code: `sad/src/model.rs`, `checks/smell.rs` (new shared classifier),
  `checks/outside_channel.rs`, `checks/doc_cap.rs`, `checks/mod.rs`,
  `checks/commented_out_code.rs`, `attribution.rs`, `check.rs`,
  `envelope.rs`, `render.rs`, `cli.rs`, `config.rs`.
- Output: additive `smell` (nullable), `severity`,
  `stats.findings_blocking`, `stats.comments_rationale`; `--strict`;
  `ok`/exit semantics change (0.2.0).
- Tests: every contract touching prose, doc cap, verdict, text mode,
  suppression.
- Docs: `docs/src/gate-model.md` (severity table, algorithms,
  dispositions, verdict), `CHANGELOG.md`, config table
  (`doc_summary_cap` sentences).
- Ticket: `sad-66v` (feature, needs-spec-change).

## Dependencies (not bundled)

- `sad-jdw` block-check precision: this change ships the check as a
  warning; the ticket is the prerequisite for restoring error severity.
- `sad-h3i` shebang/PEP 263 false positives, `sad-8ah` gitignore and
  generated-path off-ramp, `sad-6z5` non-SPDX licence headers: they are
  warning noise under the new classifier, not blockers. Land them to keep
  warning volume honest.
- A later `ddl` change for complexity escalation (see Out of scope).

## Out of scope

- NLP or model-based comment classification. Heuristics are
  deterministic and conservative; ambiguous prose is `unclassified`
  (warning), never an error.
- Reading pretender metrics from sad. Complexity escalation is an
  orchestrator rule in `ddl`: a `narration` warning may become blocking
  when a structure gate flags the same unit. The boundary stays intact.
- Remediation-outcome telemetry; the follow-up change measures
  comment-only vs code-changing fixes and the quality outcome.
- Fixing the noise tickets inline; they keep their own changes.