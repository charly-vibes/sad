# Design: gate-field-hardening

## Finding identity and the occurrence discriminator

Base identity stays `<path>::<symbol-path>::<content-hash>` so no unique ID
churns. After every check has run and findings are sorted by `(path, line,
check id)`, one pass walks the report in order and counts base identities;
the `n`th occurrence (`n >= 2`) becomes `<base>#<n>`. The first occurrence
keeps the bare base, which keeps existing baselines valid for every
non-duplicate finding.

Properties:

- Unique within a run, including collisions between different checks.
- Line-independent: inserting or deleting unrelated lines changes no ID.
- Order-sensitive only among identical bodies, where any discriminator is
  arbitrary. Deleting one of two identical comments shifts the survivor's
  discriminator; grandfathered debt still suppresses it, exactly as for any
  two distinct findings when one is fixed. The bijection the fix restores is
  between baseline entries and findings **at adoption time**, not between
  adopted debt and a later tree.

Rejected alternatives: including the line/column (breaks the stability
requirement and churns every ID on any edit above); hashing surrounding
source lines (still collides for identical context and couples an ID to
unrelated code); storing counts instead of IDs (cannot express which
occurrence was fixed and makes per-finding review impossible).

## Baseline schema migration

`sad-baseline.json` becomes `schema_version: 2`. `Baseline::load` accepts 1
and 2 and rejects anything else with the existing named-path error. Adoption
against a version-1 document skips the growth check exactly once and
persists version 2; adoption against version 2 is shrink-only as before.
Plain runs suppress exact id matches from either version, so a v1 baseline
keeps working without re-adoption (duplicate occurrences report as new debt
until adopted).

There is deliberately no `--allow-growth` flag: a durable escape hatch
would erode the invariant, while the schema bump marks exactly one
sanctioned migration.

## Scope observability

`scope::select` keeps its signature; a new `scope::select_in_scope` returns
`ScopeSelection { files, attributed, without_commit, unattributed }`.
`select` delegates to it, so existing callers and tests are untouched.

- `files_attributed`: kept because the governing revision carries the
  configured trailer.
- `files_without_commit`: kept because git has no governing revision.
- `files_unattributed`: dropped because the governing revision lacks the
  trailer.
- `--all`: every candidate is selected and no attribution is resolved, so
  all three counters are zero and `files_selected` equals the candidate
  count (with `files_excluded` accounting for globs).

The stats block gains the three counters. The text render appends the
attribution hint only when `files_selected == 0` and at least one candidate
survived exclusion — the exact case where a green run gated nothing. The
hint stays on the one line the text contract pins.

## Language boundary

`SOURCE_EXTENSIONS` and `extract::language_for_path` grow to
`rs, py, ts, js, mjs`; `.js`/`.mjs` use the pinned TypeScript grammar
(tree-sitter-typescript parses JavaScript). Discovery additionally
recognises `tsx, mts, cts, jsx` and returns them as unsupported paths
instead of dropping them. `check::run` publishes one `Warning` per path
(`rule_name = "unsupported_extension"`, `entity_id = path`) and counts
`files_unsupported`. Warnings ride the genesis envelope channel
(`Envelope::warnings`), not the data payload, so the report shape is
stable.

`.pyi` is not part of this boundary: its semantics are stub-only and stay
with `sad-7hx`.

## Bounded block predicate

Constants next to the checker:

- `MAX_PREDICATE_BYTES = 8 * 1024`
- `MAX_PREDICATE_LINES = 512`

A normalised body within both bounds is parsed once — byte-for-byte today's
behaviour. A longer body is split into chunks at line boundaries, greedily
packed up to both bounds, and each chunk is parsed; the run matches when any
chunk satisfies the predicate. The finding keeps the whole body, so its ID
does not depend on chunking.

Cost: work becomes linear in body size with a bounded parse input, removing
the superlinear error-recovery cliff (32k lines: 63s → measured under 2s).
Trade-off: a code construct spanning a chunk boundary can fail its chunk and
fall back to prose. That is fail-closed (per-node prose findings still
report the comments); it is documented in the constant's doc comment and in
`docs/src/gate-model.md`.

The regression test asserts both the bound (a 32k-line prose run completes
well under the observed cliff) and that runs below the bound keep exactly
the findings of the single-parse path.

## Verification

- `cargo test --package sad`
- `sad check --all` (self-gate) after regenerating `sad-baseline.json`
- `ah check --changes gate-field-hardening` (contracts per scenario)
- `openspec validate gate-field-hardening --strict`
- `just ci`