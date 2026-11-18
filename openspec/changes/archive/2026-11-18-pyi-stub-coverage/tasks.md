# Tasks: pyi-stub-coverage

TDD: the discovery test landed with the extension change; the extraction
test and the `language_for_path` arm land with the Python frontend half of
the same change. The spec delta is this file's sibling
`specs/comment-gate/spec.md`, and the decision record is `design.md`.

## sad-7hx — decision

- [x] 1. Record the decision in `design.md`: stubs are analysed as Python
      with tree-sitter-python, and the doc channel is unchanged (module,
      class, and function docstrings only; no stub-specific rule)

## sad-7hx — discovery

- [x] 2. Add `pyi` to `SOURCE_EXTENSIONS` (six extensions) and widen the
      `unsupported_extension` warning's supported set
- [x] 3. Red: a `.pyi` file is reported in `files`, not `unsupported`
      (`pyi_stubs_are_discovered`)

## sad-7hx — extraction

- [x] 4. Map `pyi` to the Python frontend in `language_for_path`
      (`sad/src/extract.rs`, landed with this change)
- [x] 5. Red: a stub is parsed as Python and yields the same findings as the
      equivalent `.py` source (`pyi_stubs_are_analyzed_as_python`)

## Contracts and gates

- [x] 6. Declare the espectacular contracts for both scenarios
- [x] 7. Run `cargo test --package sad`, `sad check --all`, `ah check
      --changes pyi-stub-coverage`, `openspec validate pyi-stub-coverage
      --strict`
- [x] 8. Close `sad-7hx` with evidence and deploy the spec delta