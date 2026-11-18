# Proposal: gate-skipped-sources

## WHY

A selected source file that cannot be read as UTF-8 is dropped by
`check::analyze_files` with nothing but the aggregate `stats.files_skipped`
count. The field test proved the gap: a nine-file encoding corpus with
latin-1 bytes, invalid UTF-8, and UTF-16LE `.rs` files discovered all nine,
analysed six, and printed `pass: 4 findings in 6 files` with no warning and
no path. A `chmod 000` file behaves the same.

A repo-wide pass therefore does not mean every discovered source was read,
and a corrupt or vendored-with-another-encoding file is permanently ungated
with no signal to the agent or CI consumer.

## What Changes

- Every skipped selected file SHALL produce one `unreadable_source` envelope
  warning naming its repo-relative path and a stable reason: `not valid
  UTF-8`, `permission denied`, or `file disappeared during the run`.
- `stats.files_skipped` remains the count; no new stat field is added.
- The text render SHALL append the skipped count and each path with its
  reason, so a passing one-liner is never silent about unread sources.
- The gate still does not guess an encoding and invents no findings for
  skipped content; warnings alone do not change the exit status.

## Impact

- Spec: `openspec/specs/comment-gate/spec.md` gains one requirement
  (unreadable sources are reported).
- Code: `sad/src/check.rs` (skip collection and warnings),
  `sad/src/cli.rs` (text render), module headers.
- Tests: `sad/tests/check_stats_test.rs` (counts) and
  `sad/tests/check_envelope_test.rs` (warnings, text render).
- Docs: `docs/src/gate-model.md` documents the warning channel and reasons.
- Ticket: `sad-o40`.

## Out of scope

- Encoding detection or transcoding. The gate reads UTF-8 only.
- Failing the run on warnings; skip visibility is the contract, not a new
  verdict rule.
- Unreadable *directories* stay silently skipped: discovery's deterministic
  pure walk is unchanged by this change.