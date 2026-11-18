# Proposal: skipped-directory-visibility

## WHY

`SKIPPED_DIRECTORIES` matched 14 names at any depth of the walk with no
statement of the rule and no signal when a skip hid a source file. Two
problems followed:

- **Coverage gaps.** `vendor`, `third_party`, `.mypy_cache`, `.ruff_cache`,
  `out`, `.turbo`, and `.svelte-kit` were walked, so a vendored tree or a
  bundler cache could feed third-party prose into the gate while the names
  that usually mark build output were half-covered.
- **Silent misses in the other direction.** The match is on any path
  component, so a legitimate source directory named `target/`, `build/`,
  `dist/`, `book/`, or `out/` — `src/target/foo.rs` is the ticket's example —
  is invisible to the gate with no warning at all. The verdict says "pass"
  over sources that were never read.

The skip list is therefore undocumented prevention on one side and an
invisible hole on the other.

## What Changes

- `SKIPPED_DIRECTORIES` SHALL add `vendor`, `third_party`, `.mypy_cache`,
  `.ruff_cache`, `out`, `.turbo`, and `.svelte-kit`, and the discovery doc
  comment SHALL state the list and its two rules: the names are reserved for
  build, vendor, and cache trees, and a matching name nested below the
  repository root is ambiguous because a legitimate source directory can use
  it.
- Discovery SHALL record skipped directories whose name is in the ambiguous
  subset {`target`, `build`, `dist`, `book`, `out`} and whose depth is two or
  deeper, as repo-relative paths with a trailing `/`. A root-level `target/`
  or `build/` stays silent, and no files inside a skipped directory are
  enumerated: the warning names the directory, deterministically and at no
  extra traversal cost.
- `check::run` SHALL emit one `skipped_source_directory` envelope warning per
  recorded directory, carrying the path as `entity_id`, a message naming the
  path and the skip-list name, and a remediation that offers renaming a
  hand-written source directory or keeping skip-list names for build output.
- No stats counter is added and no existing warning or finding is renumbered;
  warnings still do not change the exit status.

## Impact

- Spec: `openspec/specs/comment-gate/spec.md` gains one requirement
  (skipped source directories are observable) with three scenarios.
- Code: `sad/src/check/discovery.rs` (list, doc comment, skipped record),
  `sad/src/check.rs` (`Selection`, `skipped_directory_warnings`).
- Tests: `sad/src/check/discovery.rs` unit tests and the new
  `sad/tests/check_discovery_visibility_test.rs`. The exclusion fixture in
  `sad/tests/check_stats_test.rs` moves off `vendor/`, which is now a skipped
  name, so the exclude contract keeps its own coverage.
- Ticket: `sad-xkz`.

## Out of scope

- Failing the run on a skipped source directory; the warning reports the gap,
  the verdict stays finding-derived.
- Enumerating the files a skipped directory holds. That would walk the tree
  the gate just refused and make the warning's cost proportional to vendor
  size.
- A warning for every skipped name. `.venv`, `node_modules`, and the caches
  are unambiguously machine trees; naming them would bury the ambiguous case.
- The gitignore/generated-path off-ramp (`sad-8ah`), which decides what is
  in scope rather than what is walked.