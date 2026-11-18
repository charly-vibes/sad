# Tasks: comment-smell-triage

TDD: each area starts with a red contract test, then implementation. The
spec delta is this file's sibling `specs/comment-gate/spec.md`. Do not
bundle the noise tickets (`sad-h3i`, `sad-8ah`, `sad-6z5`): they keep
their own changes; this change only removes separator bodies from the
error class.

## sad-66v — smell kind and severity

- [ ] 1. Red: a narration comment, an apology, and an unresolvable TODO
      each report `smell`/`severity: error`; a `// Purpose:` TS header, a
      domain explanation, a banner, and a separator-only block body each
      report `warning`
- [ ] 2. Red: narration detection is conservative — a comment with no
      adjacent-token overlap and no sequencer stays `unclassified`
- [ ] 3. Implement the deterministic classifier in
      `checks/outside_channel.rs` (or a small `checks/smell.rs`), with
      separator bodies from `commented_out_code` reclassified as `banner`
- [ ] 4. Add `smell`, `severity` to `Finding`; keep IDs unchanged

## sad-66v — verdict and stats

- [ ] 5. Red: warnings-only run has `ok: true`, exit 0,
      `stats.findings` > 0, `stats.findings_blocking: 0`; one error blocks
- [ ] 6. Red: text mode prints `pass: N findings (0 blocking, N warnings)`
      for a warnings-only run and `fail:` for an error
- [ ] 7. Implement `findings_blocking`, verdict from errors, and the text
      render change
- [ ] 8. Confirm adoption still adopts warnings and errors and remains
      shrink-only

## sad-66v — rationale channel

- [ ] 9. Red: `SAFETY:`, `INVARIANT:`, `SECURITY:` with a body attribute
      to rationale, emit no finding, and increment `comments_rationale`;
      empty-body tags and `NOTE:` stay prose
- [ ] 10. Implement the `rationale` class and stat

## sad-66v — doc cap in sentences

- [ ] 11. Red: a one-sentence summary wrapped over four lines passes; a
      two-sentence lead warns; structured sections remain uncapped
- [ ] 12. Replace rendered-line counting with sentence counting;
      keep `doc_summary_cap` key/default, update its doc and validation

## sad-66v — remediation and docs

- [ ] 13. Red: each smell kind carries its disposition template; no
      informative smell offers a bare delete
- [ ] 14. Update `docs/src/gate-model.md` (classification table,
      dispositions, verdict, config table) and `CHANGELOG.md`; bump to
      0.2.0

## Contracts and gates

- [ ] 15. Declare espectacular contracts for every scenario in the delta
      under `.espectacular/changes/comment-smell-triage/comment-gate/`
- [ ] 16. Run `cargo test --package sad`, `sad check --all`, `ah check
      --changes comment-smell-triage --run-tests`, `openspec validate
      comment-smell-triage --strict`, `just ci`, `typos`
- [ ] 17. Archive the change, deploy the spec, and close `sad-66v` with
      evidence

## Follow-ups (separate changes)

- [ ] Noise prerequisites: `sad-h3i`, `sad-8ah`, `sad-6z5`
- [ ] `ddl` complexity escalation: a `narration` warning becomes blocking
      when a structure gate flags the same unit
- [ ] Remediation-outcome telemetry: comment-only vs code-changing fixes,
      to test whether the gate improves quality rather than style