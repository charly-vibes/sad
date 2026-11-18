# Tasks: comment-smell-triage

TDD: each area starts with a red contract test, then implementation. The
spec delta is this file's sibling `specs/comment-gate/spec.md`; the
corpus evidence is `evidence.md`. Do not bundle the noise tickets
(`sad-h3i`, `sad-8ah`, `sad-6z5`) or the precision ticket (`sad-jdw`);
they keep their own changes.

## sad-66v — finding model

- [ ] 1. Red: `missing_header`, `unresolvable_marker`, and
      `suppression_cap_exceeded` carry `smell: null` and
      `severity: error`; `commented_out_code` and `doc_cap` are warnings
- [ ] 2. Red: every finding serializes both fields, snake_case, `smell`
      null for structural findings
- [ ] 3. Implement `severity` on `Finding` and the nullable `smell`

## sad-66v — shared classifier

- [ ] 4. Red: narration fixtures — two distinctive-token matches, a
      sequencer plus named call, a three-node run, and a module-level
      statement
- [ ] 5. Red: apology fixtures — bare "temporary" passes, `temporary
      fix`/`workaround`/`hack:` errors; a resolvable marker passes
- [ ] 6. Red: banner bodies (including separator block bodies) are
      `banner` warnings
- [ ] 7. Implement `checks/smell.rs` and call it once from
      `check::collect_findings` over all findings, grouping maximal
      adjacent prose runs; block findings included
- [ ] 8. Verify per-node IDs are unchanged and every member of a run
      receives the run's smell/severity

## sad-66v — rationale channel

- [ ] 9. Red: `SAFETY:`, `INVARIANT:`, `SECURITY:`, `WHY:` with a body
      are rationale and counted; empty tags and `NOTE:` stay prose
- [ ] 10. Red: `//! SAFETY:` is header and `/// WHY:` is doc under the
      precedence chain
- [ ] 11. Implement the rationale class, `comments_rationale`, and the
      precedence chain

## sad-66v — doc cap

- [ ] 12. Red: wrapped one-sentence passes; two sentences warn;
      `e.g.`, `i.e.`, `v0.1`, `Node.js`, URLs, and inline code do not
      split
- [ ] 13. Replace rendered-line counting with the documented sentence
      algorithm; keep the `doc_summary_cap` key/default and update its
      validation text

## sad-66v — verdict and suppression

- [ ] 14. Red: warning-only run is `ok: true`, exit 0,
      `findings_blocking: 0`; one error fails; `--strict` fails the
      warning-only run
- [ ] 15. Red: text mode prints the blocking/warning counts and the
      strict failure; no-finding output is unchanged
- [ ] 16. Implement `findings_blocking`, severity-based verdict,
      `--strict`, and the render change
- [ ] 17. Red: a `sad:ignore comment_outside_channel` directive is
      counted but does not withhold an error smell
- [ ] 18. Confirm adoption adopts warnings and errors and remains
      shrink-only; adoption grandfathers severity drift

## sad-66v — remediation and docs

- [ ] 19. Red: each smell kind carries its disposition template; no
      informative smell offers a bare delete
- [ ] 20. Update `docs/src/gate-model.md` (severity table, algorithms,
      dispositions, verdict, config table), `CHANGELOG.md`, and the
      crate version to 0.2.0
- [ ] 21. Keep `evidence.md` in the change and reference it from the
      proposal

## Contracts and gates

- [ ] 22. Declare espectacular contracts for every scenario in the delta
      under `.espectacular/changes/comment-smell-triage/comment-gate/`
- [ ] 23. Run `cargo test --package sad`, `sad check --all`, `ah check
      --changes comment-smell-triage --run-tests`, `openspec validate
      comment-smell-triage --strict`, `just ci`, `typos`
- [ ] 24. Archive the change, deploy the spec, and close `sad-66v` with
      evidence

## Follow-ups (separate changes)

- [ ] `sad-jdw`: block-check precision; restore error severity after its
      precision corpus passes
- [ ] Noise: `sad-h3i`, `sad-8ah`, `sad-6z5`
- [ ] `ddl` complexity escalation: a `narration` warning becomes blocking
      when a structure gate flags the same unit
- [ ] Remediation-outcome telemetry: comment-only vs code-changing fixes,
      complexity of touched units, defect/rework rate; success target in
      `design.md` "How we will know"