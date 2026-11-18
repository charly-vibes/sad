# Proposal: gate-field-hardening

## WHY

The v0.1 gate was field-tested against the charly-vibes fleet (48 repos,
3098 files) and six synthetic corpora. Three defects break the gate's own
contract and two silently shrink its coverage:

- Finding IDs are not unique: two identical prose bodies in the same
  file+symbol share one ID, so `sad-baseline.json` stores one entry for two
  findings (`sad/tests/extract_test.rs` reports 297 findings with 296 unique
  IDs today). The envelope promises stable IDs; consumers may assume
  uniqueness, and adoption's id set under-counts debt. The shrink-only guard
  also makes the fix impossible without a baseline migration affordance.
- A default run that selects zero files prints `pass: 0 findings in 0 files`
  — indistinguishable from a genuinely clean run. The selection is
  spec-conforming (agent-attribution scoping); the defect is observability.
- `.js`/`.mjs` sources (843 + 64 files in the fleet) are gated by nobody,
  while `.tsx`/`.mts`/`.cts`/`.jsx` vanish without a warning.
- `commented_out_code` parses a whole maximal prose run in one tree-sitter
  parse: 32k adjacent comments take 63s, and memory scales with the run.

## What Changes

- **Finding identity** gains a deterministic occurrence discriminator:
  the base identity stays `<path>::<symbol-path>::<content-hash>`; a second
  and later finding sharing that base identity in one run appends `#<n>`
  in report order. `sad-baseline.json` moves to `schema_version: 2`; a
  version-1 baseline is still honoured for suppression and is migrated by
  the next `--adopt` (that one adoption may add ids; every later adoption
  is shrink-only again).
- **Scope observability**: the stats block reports
  `files_attributed`, `files_without_commit`, and `files_unattributed`, and
  a default-mode run that selects zero files states the attribution
  decision and the widening paths (`--all`, or commit with the configured
  trailer) in the same one-line text render. Selection semantics are
  untouched.
- **Language boundary**: `.js` and `.mjs` join `.rs`/`.py`/`.ts` and route
  through the TypeScript frontend; `.tsx`, `.mts`, `.cts`, and `.jsx` are
  discovered, skipped, counted in `files_unsupported`, and reported as
  envelope warnings naming each path. `.pyi` stays with `sad-7hx`.
- **Bounded block predicate**: a normalised run is parsed in one unit while
  it stays within 8192 bytes and 512 lines; a longer run is parsed in
  bounded chunks cut at line boundaries, and any matching chunk keeps the
  whole run one `commented_out_code` finding with the same body hash.
  Findings below the bound are unchanged.

## Impact

- Specs: `openspec/specs/comment-gate/spec.md` gains two requirements
  (finding identity, scope observability) and modifies two (block-scoped
  finding, language frontends).
- Code: `sad/src/check.rs`, `sad/src/check/discovery.rs`, `sad/src/model.rs`,
  `sad/src/baseline.rs`, `sad/src/scope.rs`, `sad/src/envelope.rs`,
  `sad/src/cli.rs`, `sad/src/extract.rs`,
  `sad/src/checks/commented_out_code.rs`.
- Baseline: `sad-baseline.json` is regenerated under the new ID scheme as
  part of this change.
- Docs: `docs/src/gate-model.md` gains the extension set, the occurrence
  discriminator, the scope counters, and the predicate bound.
- Tickets: `sad-kte`, `sad-04v`, `sad-54i`, `sad-suj`.

## Out of scope

- The remaining field-test tickets (`sad-h3i`, `sad-o40`, `sad-8ah`,
  `sad-nw1`, `sad-6z5`, `sad-7hx`, `sad-xkz`) keep their own change: a
  delta cannot deploy before its tests exist, so bundling them here would
  ship unverified requirements. Their deltas share this change's sections
  and follow the same review path.
- No JSX/TSX parsing, no `.mts`/`.cts` frontend (separate grammar
  decision), no `.pyi` coverage.
- No working-tree evidence in attribution, no new attribution heuristics.
- No change to the shrink-only invariant beyond the one-time v1 migration.