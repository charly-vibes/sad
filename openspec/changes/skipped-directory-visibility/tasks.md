# Tasks: skipped-directory-visibility

TDD: the red discovery and envelope tests landed with the implementation;
the spec delta is this file's sibling `specs/comment-gate/spec.md`.

## sad-xkz — documented skip list

- [x] 1. Add `vendor`, `third_party`, `.mypy_cache`, `.ruff_cache`, `out`,
      `.turbo`, and `.svelte-kit` to `SKIPPED_DIRECTORIES` and rewrite the
      doc comment to state the list and its two rules
- [x] 2. Red: the seven added names are skipped by discovery and report no
      file (`added_skip_names_are_skipped`)

## sad-xkz — nested skips are named

- [x] 3. Red: `src/target/foo.rs` produces no finding and exactly one
      `skipped_source_directory` warning naming `src/target/` and `target`
      (`nested_skip_list_directory_is_named`)
- [x] 4. Red: a root-level `target/foo.rs` produces no warning and leaves the
      analyzed count unchanged (`root_build_output_stays_silent`)
- [x] 5. Implement `Discovery::skipped`: ambiguous names at depth two or
      deeper, repo-relative, trailing `/`, sorted, no file enumeration
- [x] 6. Extend `Selection` and emit the warnings from `check::run`; no stats
      counter, no verdict change
- [x] 7. Move the exclusion fixture off `vendor/`, now a skipped name, and
      assert the exclude accounting still holds

## sad-xkz — text-mode parity

- [x] 8. Red: the one-line render names the skipped directory alongside the
      unreadable-source suffix (`nested_skip_list_directory_is_named_in_text_mode`)
- [x] 9. Implement the skipped-directory suffix in `render::skipped_suffix`
      without changing a run that has no such warnings

## Contracts and gates

- [x] 10. Declare the espectacular contracts for the four scenarios
- [x] 11. Run `cargo test --package sad`, `sad check --all`, `ah check
      --changes skipped-directory-visibility`, `openspec validate
      skipped-directory-visibility --strict`
- [ ] 12. Close `sad-xkz` with evidence and deploy the spec delta
- [x] 13. Declare the `text-mode-names-a-skipped-source-directory` contract
      (flags `nested_skip_list_directory_is_named_in_text_mode`) for the
      text-mode scenario, including the unchanged one-liner when a tree has
      no ambiguous skip