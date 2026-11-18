# Tasks: gitignore-aware-discovery

TDD: the red tests land before implementation. The spec delta is this
change's `specs/comment-gate/spec.md`; ticket `sad-z6q`, decision `sad-8ah`
(option a).

## sad-z6q — red phase

- [x] 1. Red: `gitignored_sources_are_not_gated` — a `.git` tree with a root
      `.gitignore` covering `src/generated/` omits `src/generated/api.ts`
      from discovery and from a `sad check --all` finding list
- [x] 2. Red: `not_gated_source_in_non_ignored_path_stays_gated` — a
      generated-looking path with no matching rule is discovered and yields
      `comment_outside_channel`
- [x] 3. Red: `negated_ignore_rule_reincludes_entry` — `!src/generated/keep.ts`
      after `src/generated/*.ts` re-includes that one entry
- [x] 4. Red: `directory_only_rule_leaves_same_named_file` — a trailing-`/`
      rule prunes the directory and leaves the same-named regular file gated
- [x] 5. Red: `nested_ignore_file_overrides_root` — `src/.gitignore` negates a
      root ignore rule for `src/keep.ts`
- [x] 6. Red: `no_git_work_tree_ignores_no_ignore_file` — a `.gitignore`
      without `.git` changes nothing

## sad-z6q — implementation

- [x] 7. Detect `<repo>/.git` (directory or worktree file), never read its
      contents, never search upward
- [x] 8. Read root and reached nested `.gitignore` files during the walk,
      maintaining a per-directory rule stack
- [x] 9. Compile rules to `(anchored, directory_only, negated, segments)` with
      the closed grammar: basename-at-any-depth, `/` anchoring, trailing `/`,
      `!`, `*`/`?` within a segment, `**` whole-segment including trailing
      `/**`, literal classes/braces/backslash, CRLF and blank trimming
- [x] 10. Evaluate rules per entry before extension filtering; prune ignored
      directories so negation cannot re-include below them
- [x] 11. Total fallbacks: missing, unreadable, or non-UTF-8 file contributes
      no rules and never fails discovery
- [x] 12. No new counter, warning, or exemption count; keep `SKIPPED_DIRECTORIES`
      and `sad.toml` excludes untouched

## sad-z6q — docs and evidence

- [x] 13. Author the spec delta with one added requirement and six scenarios
- [x] 14. Declare one `.espectacular/changes/gitignore-aware-discovery/comment-gate/`
      contract per scenario, `flags` matching the planned test function name
- [x] 15. Add the `Ignore files` subsection to `docs/src/gate-model.md`
- [x] 16. Record the v0.2.0 bullet in `CHANGELOG.md` naming `sad-z6q`
- [x] 17. Run `cargo test --package sad`, `sad check --all`, the change
      contracts, and `openspec validate gitignore-aware-discovery --strict`
- [x] 18. Close `sad-z6q` and deploy the spec delta