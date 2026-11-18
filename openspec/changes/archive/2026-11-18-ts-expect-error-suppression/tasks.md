# Tasks: ts-expect-error-suppression

TDD: the red contract test lands with the spec delta in this change's
sibling `specs/comment-gate/spec.md`, then the implementation turns it
green.

## sad-nw1 — @ts-expect-error

- [x] 1. Red: a comment starting with `@ts-expect-error` is
      suppression-class, appears once in `suppression_debt` with form
      `ts_expect_error` and no check id, and emits no
      `comment_outside_channel` finding
- [x] 2. Red: the existing every-form debt test covers the new form in
      source order
- [x] 3. Add `SuppressionForm::TsExpectError` (serde `ts_expect_error`)
      and its snake-case label in `checks::suppression`
- [x] 4. Implement `parse_ts_expect_error` and wire it into
      `parse_directive` beside `parse_ts_ignore`

## Contracts and gates

- [x] 5. Declare the espectacular contract for the new scenario under
      `.espectacular/changes/ts-expect-error-suppression/comment-gate/`
- [x] 6. Run the targeted cargo tests and
      `openspec validate ts-expect-error-suppression --strict`