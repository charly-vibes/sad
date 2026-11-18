# Proposal: ts-expect-error-suppression

## WHY

`attribution::parse_directive` recognises `noqa`, `type: ignore`,
`@ts-ignore`, `eslint-disable`, and `sad:ignore`. TypeScript's current
recommended directive is `@ts-expect-error`; it falls through to prose, so
the fleet test (`sad-nw1`) reports roughly 80 `comment_outside_channel`
findings whose text begins with that directive. Adopting repos cannot record
the directive their compiler requires without producing a documentation
violation, and the deployed requirement's closed form list enshrines the
deprecated `@ts-ignore`.

## What Changes

- **`@ts-expect-error` joins the recognised directive forms.**
  `SuppressionForm::TsExpectError` serialises as `ts_expect_error`,
  `parse_ts_expect_error` parses it unscoped (check `None`) like
  `@ts-ignore`, and `parse_directive` tries it beside the other forms.
- **Exempt-but-counted.** The directive node attributes to suppression,
  appears in `suppression_debt`, and never in
  `comment_outside_channel`. It scopes to the next line, so it withholds no
  other finding.

## Impact

- Spec: MODIFIES `Suppression debt is exempt-but-counted`.
- Code: `sad/src/model.rs`, `sad/src/attribution.rs`,
  `sad/src/checks/suppression.rs` (snake-case form label).
- Tests: `sad/tests/check_suppression_test.rs`,
  fixture `sad/tests/fixtures/checks/suppression/expect_error.ts`.
- Ticket: `sad-nw1`.