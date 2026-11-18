# Tasks: gate-field-hardening

TDD: each ticket starts with a red test, then implementation; the spec
delta lands first (this file's sibling `specs/comment-gate/spec.md`).
Baseline regeneration is part of `sad-kte`, not a separate ticket.

## sad-kte — unique finding identity and baseline v2

- [x] 1. Red: a fixture tree with two identical prose bodies in one
      file+symbol reports two distinct IDs (`<base>`, `<base>#2`) and
      `sad-baseline.json` adopts both entries
- [x] 2. Red: a version-1 baseline built before the fix still suppresses
      its exact ids and the next `--adopt` migrates it to
      `schema_version: 2`; a second adoption with a larger id set is
      refused (shrink-only restored)
- [x] 3. Implement the post-sort occurrence discriminator in
      `check::collect_findings`; IDs of non-duplicate findings do not change
- [x] 4. Implement `Baseline` schema 2: accept 1 and 2 on load, migrate on
      adoption, reject unknown versions with the named-path error
- [x] 5. Regenerate `sad-baseline.json` (`sad check --all --adopt`) and
      confirm `sad check --all` reports 0 new findings with
      `findings_suppressed` equal to the unique finding count

## sad-04v — scope decisions are observable

- [x] 6. Red: `scope_test` pins `select_in_scope` counts for attributed,
      no-commit, and unattributed files, and `--all`
- [x] 7. Red: an end-to-end default run that selects zero files is
      distinguishable from a clean run in text mode and carries
      `files_attributed`/`files_without_commit`/`files_unattributed` in
      `data.stats`
- [x] 8. Implement `ScopeSelection` + counters, stats fields, and the
      one-line widening hint; selection semantics unchanged

## sad-54i — JavaScript sources and the visible boundary

- [x] 9. Red: a tree with `.js` and `.mjs` prose yields findings via the
      TypeScript frontend; `.tsx`/`.mts`/`.cts`/`.jsx` produce
      `unsupported_extension` warnings plus `files_unsupported` and no
      findings
- [x] 10. Implement the extension set, `language_for_path` routing,
      unsupported-extension discovery, and envelope warnings
- [x] 11. Document the extension set and the JSX/TSX exclusion in
      `docs/src/gate-model.md`

## sad-suj — bounded block predicate

- [x] 12. Red: a 32k-line contiguous prose run completes under the
      regression bound and reports the same per-node prose findings as
      today; a code chunk past the bound still yields one block finding
- [x] 13. Implement `MAX_PREDICATE_BYTES` / `MAX_PREDICATE_LINES` chunking
      in `commented_out_code::Checker::parses_as_code`; document the
      fail-closed chunk-boundary trade-off

## Contracts and gates

- [x] 14. Declare espectacular contracts for every scenario in the delta
      under `.espectacular/changes/gate-field-hardening/comment-gate/`
- [x] 15. Run `cargo test --package sad`, `sad check --all`, `ah check
      --changes gate-field-hardening`, `openspec validate
      gate-field-hardening --strict`, `just ci`, `typos`
- [x] 16. Close `sad-kte`, `sad-04v`, `sad-54i`, `sad-suj` with evidence;
      note the amended `sad-kte` acceptance (per-adoption bijection, not
      post-deletion re-report) in that ticket