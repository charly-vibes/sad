# comment-gate specification delta

## MODIFIED Requirements

### Requirement: Suppression debt is exempt-but-counted

The system SHALL report every suppression directive (`# noqa`,
`# type: ignore`, `// @ts-ignore`, `// @ts-expect-error`,
`// eslint-disable-*`, or `sad:ignore <check-id> <reason>`) as a
`suppression_debt` entry (file, line, directive, check) in the envelope
instead of a prose finding for the directive node itself. A directive
scopes to exactly one check and SHALL NOT withhold any other finding; an
error-severity finding remains blocking. WHEN a `suppression_cap` is
configured and a single analyzed file's debt exceeds it, the system SHALL
emit `suppression_cap_exceeded`. The default SHALL be no cap.

#### Scenario: noqa counted, not flagged

- **WHEN** a Python line carries `# noqa`
- **THEN** the envelope contains a `suppression_debt` entry for it and
  no `comment_outside_channel` finding

#### Scenario: @ts-expect-error is suppression debt

- **WHEN** a TypeScript line carries
  `// @ts-expect-error the call is deliberately loose` and no other
  directive
- **THEN** the envelope contains one `suppression_debt` entry for it with
  form `ts_expect_error` and no scoped check id, and no
  `comment_outside_channel` finding is emitted for the directive node

#### Scenario: Cap breach

- **WHEN** `suppression_cap = 5` is configured and a file carries 6
  suppressions
- **THEN** `suppression_cap_exceeded` is emitted; with the default (no
  cap) it is not

#### Scenario: Directive does not silence an error smell

- **WHEN** a file carries `sad:ignore comment_outside_channel reason`
  next to a narration comment
- **THEN** the directive is counted in `suppression_debt`, the narration
  finding remains with `severity: error`, and the run fails