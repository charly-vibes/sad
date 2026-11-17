# Tasks: specify-gate-objective

Implementation follows TDD (red→green per task; refactor commits
separate). Nothing here runs until the proposal is approved and
`sad-lcy` moves to implementation. Validation command for every task:
`cargo test` (plus `just ci` at gates).

1. [ ] Scaffold `sad` CLI skeleton + `sad.toml` config parsing
       (check toggles, summary cap, marker pattern, suppression cap,
       require_header, excludes, attribution source) — config fixture
       tests first
2. [ ] Vendor/pin tree-sitter grammars (Rust, Python, TypeScript —
       pretender's known-good pins) + comment/doc-node extraction with
       per-language parse fixtures (include CRLF files and prose inside
       string literals as negative cases)
3. [ ] Channel attribution engine (header / doc / marker / suppression /
       exemption / prose) — table-driven attribution tests per language
4. [ ] `comment_outside_channel` + stable finding IDs
       `path::symbol-path::content-hash` (drift tests: move comment,
       edit surrounding code — ID survives; edit comment — new ID)
5. [ ] `doc_summary_cap_exceeded` — first-blank-line summary-lead
       detection; cap knob; structured-section pass cases (Rust `#`
       sections, numpy `:param:`, TSDoc `@throws`)
6. [ ] `unresolvable_marker` — `marker_prefix` knob (default `see`) +
       ID pattern; beads JSONL resolver; openspec resolver with empty-
       dir no-op; absent-source no-op; lookalike non-marker negative test
7. [ ] `missing_header` — order-free keyword detection; require_header
       default-on only in sad's repo config
8. [ ] `suppression_debt` counting (all directive forms + check-scoped
       `sad:ignore <check-id> <reason>`) + optional per-file
       `suppression_cap` with `suppression_cap_exceeded`
9. [ ] Exemptions: SPDX block, `@generated` regions, exclude globs;
       exemption stats in envelope
10. [ ] Agent-attribution scoping: `Assisted-by:` trailer parsing,
       default vs `--all`; explicit-marker-only (no git inference)
11. [ ] Genesis Check envelope output + hardcoded per-check
       `RemediationEntry` templates (golden-file tests)
12. [ ] Baseline: `--adopt`, grandfather suppression, shrink-only
       invariant (grow = gate failure)
13. [ ] Self-host: adopt gate in sad's own repo (`require_header = true`),
       clear all findings, commit `sad.toml` + baseline
14. [ ] Docs: update `docs/src/gate-model.md` to v2 (summary cap
       replaces ≤3-line cap; header dogfood-default; attribution
       scoping; suppression cap knob); refresh getting-started pre-hook
       + CI snippets
15. [ ] Gates: declare espectacular (ah) contract tests for the
       deployed spec, `ah check` green, `just ci` green, `prettender
       check --mode gate` green, `typos` green
16. [ ] Close-out: comment validation telemetry plan on bd `sad-lcy`,
       export `.beads/issues.jsonl`, file follow-up ticket for
       commented-out-code class (deferred) + gemini canonical re-sync

Dependencies: 1→2→3 are serial; 4–12 depend on 3 and can proceed in
parallel pairs; 13–16 are serial finish. Task 14 can start after design
approval (docs-only) in parallel with implementation.
