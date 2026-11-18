# Tasks: gate-skipped-sources

TDD: the red tests land first, then the implementation; the spec delta is
this file's sibling `specs/comment-gate/spec.md`.

## sad-o40 — skipped sources are reported, not silent

- [x] 1. Red: a repo with a valid `.rs`, a latin-1 `.rs`, and an invalid
      UTF-8 `.rs` reports `files_skipped: 3`-minus-the-valid count and one
      `unreadable_source` warning per skipped file with path and reason
- [x] 2. Red: text mode names the skipped count and every path with its
      reason instead of printing a bare `pass`
- [x] 3. Red: a run with no skipped files emits no `unreadable_source`
      warning and its text render is unchanged
- [x] 4. Implement skip collection (`path` + stable reason) and
      `unreadable_source` envelope warnings in `check::run`
- [x] 5. Implement the text-render suffix in `cli::check_text`
- [x] 6. Update module headers and `docs/src/gate-model.md`

## Contracts and gates

- [x] 7. Declare the espectacular contract for the skip-reporting scenario
- [x] 8. Run `cargo test --package sad`, `sad check --all`, `ah check
      --changes gate-skipped-sources`, `openspec validate
      gate-skipped-sources --strict`, `just ci`, `typos`
- [x] 9. Close `sad-o40` with evidence and deploy the spec delta