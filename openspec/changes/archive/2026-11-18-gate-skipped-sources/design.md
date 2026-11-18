# Design: gate-skipped-sources

## Channel: envelope warnings, not a new stat or exemption class

The envelope already exposes the aggregate (`stats.files_skipped`) and the
per-path channel (`warnings`) used for unsupported extensions. Reusing
`warnings` with `rule_name = "unreadable_source"` gives JSON consumers one
mechanism for both kinds of coverage loss, names the path via `entity_id`,
and needs no new stat field or exempt-file bookkeeping. A whole-file
exemption class was rejected: an unreadable file was never analysed, so
calling it "exempt" would conflate an I/O failure with a policy decision.

## Reason classification

`std::io::ErrorKind` maps to three stable phrases (`InvalidData`,
`PermissionDenied`, `NotFound`); anything else keeps the operating-system
error text. The reason is stable across machines for the common cases and
never derived from a locale.

## Render shape

The one-line render appends `; skipped <n> files: <path> (<reason>), ...`.
It reads the same envelope warnings the JSON consumer reads, so text mode
cannot drift from the machine contract. Warnings remain non-fatal: the
verdict is still derived only from findings, and a warning-bearing pass is
explicit about what it did not judge.

## Rejected alternatives

- Failing the run whenever a file is skipped: converts an I/O condition
  into gate debt and makes a legitimate vendored blob block CI.
- Adding `files_unreadable` beside `files_skipped`: two counters for one
  population invites drift.
- Silent aggregate-only visibility: the field test showed the text render
  is what humans and agents read; a count without paths is not actionable.