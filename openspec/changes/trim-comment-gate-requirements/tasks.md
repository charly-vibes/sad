# Tasks: trim-comment-gate-requirements

Documentation-only change: no implementation work, no code. Every task
runs at apply/archive time.

1. [ ] Diff the delta's scenario titles for the three MODIFIED
       requirements against the deployed spec: identical set, no
       renames, and every deployed requirement title byte-identical
       (the 24 espectacular contracts map by scenario slug). New
       scenario titles are additive only.
2. [ ] Run `openspec validate trim-comment-gate-requirements --strict
       --no-interactive --json`: zero warnings, including no overlong
       requirement prose.
3. [ ] Archive with `openspec archive trim-comment-gate-requirements`;
       confirm the rebuilt deployed spec keeps the requirement set and
       existing scenario titles unchanged, then run `openspec validate
       comment-gate --strict --json` on it: zero warnings.
4. [ ] Move the espectacular contract mappings unchanged: existing
       `.espectacular/comment-gate/*.toml` contract ids (scenario slugs)
       stay exactly as they are; add contract files for any new
       scenario titles as test coverage lands, authored by the test
       owner, not by this change.
5. [ ] Run `ah check --changes trim-comment-gate-requirements` before
       archive and `ah check` after it; both must be green with no
       orphaned, renamed, or missing contract ids.