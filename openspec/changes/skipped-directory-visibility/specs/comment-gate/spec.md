## ADDED Requirements

### Requirement: Skipped source directories are observable

Discovery SHALL document its skip list as build, vendor, and cache names that a source directory may use. Nested `target`, `build`, `dist`, `book`, or `out` directories at depth two or deeper SHALL each produce one `skipped_source_directory` warning naming the path; root-level matches SHALL stay silent. Text mode SHALL name each skipped directory path in its own suffix and leave skip-free runs unchanged. Skipped directories SHALL NOT be enumerated and warnings SHALL NOT change the exit status.

#### Scenario: Nested skip-list directory is named

- **WHEN** a tree contains `src/target/foo.rs` and `sad check --all` runs
- **THEN** `src/target/` is skipped without analysis, `foo.rs` yields no
  findings, and exactly one `skipped_source_directory` warning carries the
  path `src/target/` and names the skip-list name `target`

#### Scenario: Root build output stays silent

- **WHEN** a tree contains `target/foo.rs` and a non-skipped source, and
  `sad check --all` runs
- **THEN** `target/` is skipped with no `skipped_source_directory` warning
  and the analyzed count still covers the non-skipped source

#### Scenario: Added skip names are skipped

- **WHEN** a tree contains `vendor/`, `third_party/`, `.mypy_cache/`,
  `.ruff_cache/`, `out/`, `.turbo/`, and `.svelte-kit/` directories holding
  source files
- **THEN** discovery traverses none of them, reports no source file from
  them, and records no ambiguous nested directory

#### Scenario: Text mode names a skipped source directory

- **WHEN** a tree contains `src/target/foo.rs` and a non-skipped source, and
  `sad check --all` runs without `--json`
- **THEN** the one-line render appends a suffix naming every skipped
  directory path, here `src/target/`, and keeps the analyzed count; a tree
  without ambiguous skips renders the unchanged one-liner