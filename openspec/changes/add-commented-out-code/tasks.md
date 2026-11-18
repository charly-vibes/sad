# Tasks: add-commented-out-code

Implementation of bd `sad-wfj`. TDD: every step starts red, then goes
green; refactors are separate commits. Nothing here is started. The
spec delta in `specs/comment-gate/spec.md` lands before code; the
authoritative design is
`.wai/projects/sad/designs/2026-11-17-commented-out-code-finding-class.md`.

## Fixtures and red tests

- [ ] 1. Add fixtures for each positive: Rust 4-line `//` fn (plain and
      `#[test]`), Rust `/* let x = 1; */`, gutter-decorated Rust
      `/* * fn f() {} */`, Python adjacent `#` loop, Python
      `# x = f()`, TS `// const x = await load();`, TS block
      `/* fn f(){g()} */`; assert one `commented_out_code` finding each
- [ ] 2. Add near-miss fixtures and assert prose-only: Rust prose,
      `TODO:`, URL, `// Returns Option<String>`, `// let x =`,
      `// ls -la`, `// 5`, `// foo`, `// fn f();`; Python
      `# list[str] | None`, `# TODO: refactor`; TS `// Note: retry();`
- [ ] 3. Add channel-exclusion fixtures: Rust `///` doc with a fenced
      code block, `//!` header with code-looking lines, Python
      docstring — assert zero findings for those nodes
- [ ] 4. Add normalization fixtures: CRLF copies of each positive
      (including `\r` before `*/`), gutter variants, ragged indentation
      — assert identical kind and identical ID per body
- [ ] 5. Add mixed-fixture counts: 12-line dead block, prose, 3-line
      dead block, 2 adjacent prose → 2 dead + 3 prose; then the same
      fixture with the knob off → 18 prose + 0 dead (never silence)
- [ ] 6. Add ID-drift tests: insert 10 lines above a block (ID stable),
      then edit one body character (ID changes); adopt-on/rerun-off
      shows prose IDs surviving

## Predicate

- [ ] 7. Add `FindingKind::CommentedOutCode` beside
      `comment_outside_channel`; wire remediation lookup and envelope
      emission without touching `AttributionClass`
- [ ] 8. Transcribe the per-language `ROOT_ALLOW`, `SHAPE`, and
      `ROOT_DENY` tables against the pinned grammar versions; unknown
      kinds reject; unit-test each table row directly and pin the
      grammar version in a comment/test
- [ ] 9. Implement the predicate: parse (same frontend as attribution),
      reject on `ERROR`/`is_missing`, require non-empty named root
      children, `ROOT_ALLOW` on every root child, at least one `SHAPE`
      node, `ROOT_DENY` outranking `SHAPE`
- [ ] 10. Implement normalization in the fixed order: line endings + BOM,
      marker stripping per kind (Rust/TS `//`+`/`|`!`, `/*`+`*`|`!` and
      trailing `*/`, gutters, Python `#`), no `#` stripping inside
      Rust/TS bodies, common-prefix dedent, trailing-whitespace trim,
      blank trim, `\n` join, empty → prose

## Emission wiring

- [ ] 11. Implement block grouping over finding-eligible nodes
      (`next.line <= prev.line + prev.text.lines().count()`); split on
      non-eligible node, code line, blank line, and symbol boundary;
      anchor at the first node with its enclosing symbol path (`-`
      file-scoped)
- [ ] 12. Wire emission: for each matched block emit one
      `commented_out_code` finding with the block body hash, and
      exclude its member nodes from `comment_outside_channel`; leave
      per-node emission untouched on predicate miss
- [ ] 13. Add the `commented_out_code` suppression id so one directive
      scopes a block, and count it as suppression debt like any other
      check
- [ ] 14. Add `[checks] commented_out_code` (default `true`) to config,
      parsing, defaults, and config fixtures; knob off restores per-node
      prose emission

## Remediation and docs

- [ ] 15. Add the hardcoded remediation template for the kind: command
      `git log -p --follow <path>` rendered at emission, description
      "Delete the commented-out code. If it is still needed, recover it
      from VCS history; never restore it as a comment."; golden-file
      test; not configurable, never executed
- [ ] 16. Update `docs/src/gate-model.md` (new check, default-on
      toggle, block-scoped emission, remediation) and the suppression-id
      list; note the known plain-comment false positive
- [ ] 17. Add `ROOT_ALLOW`/`SHAPE`/`ROOT_DENY` provenance notes
      (pinned grammar versions, fail-closed policy) next to the tables
      so a grammar bump has one audit point

## Contracts and gates

- [ ] 18. Declare espectacular (`ah`) contract tests for every scenario
      in this change's delta (both requirements) in
      `.espectacular/changes/add-commented-out-code/comment-gate/`;
      `ah check --changes add-commented-out-code --run-tests` green
- [ ] 19. Run `openspec validate add-commented-out-code --strict`,
      `just ci`, and `typos`; confirm no unrelated behavior moved
- [ ] 20. Report baseline impact in the ticket: prose IDs unchanged,
      multi-node block IDs new, knob-flip re-adoption required; close
      `sad-wfj` with evidence
