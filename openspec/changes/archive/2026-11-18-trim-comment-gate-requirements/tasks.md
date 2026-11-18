# Tasks: trim-comment-gate-requirements

Documentation-only change: no implementation work, no code. Every task
runs at apply/archive time.

1. [x] Diff the delta's scenario titles for the three MODIFIED
       requirements against the deployed spec: identical set, no
       renames, and every deployed requirement title byte-identical
       (the 24 espectacular contracts map by scenario slug). New
       scenario titles are additive only.
2. [x] Run `openspec validate trim-comment-gate-requirements --strict
       --no-interactive --json`: zero warnings, including no overlong
       requirement prose.
3. [x] Archive with `openspec archive trim-comment-gate-requirements`;
       confirm the rebuilt deployed spec keeps the requirement set and
       existing scenario titles unchanged, then run `openspec validate
       comment-gate --strict --json` on it: zero warnings.
4. [x] Move the espectacular contract mappings unchanged: existing
       `.espectacular/comment-gate/*.toml` contract ids (scenario slugs)
       stay exactly as they are; add contract files for any new
       scenario titles as test coverage lands, authored by the test
       owner, not by this change.
5. [x] Run `ah check --changes trim-comment-gate-requirements` before
       archive and `ah check` after it; both must be green with no
       orphaned, renamed, or missing contract ids.
## Completion (2026-11-18)

Applied and archived. `openspec validate trim-comment-gate-requirements
--strict` was clean; the overlay contracts restaged the five scenarios
whose bodies changed and added contracts for the five new scenarios,
mapped to existing tests. `openspec validate comment-gate --strict`
reports zero warnings after archive; existing contract ids are
unchanged and `ah check` is green.
