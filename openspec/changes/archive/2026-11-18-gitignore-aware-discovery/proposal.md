# Proposal: gitignore-aware-discovery

## WHY

Discovery is deliberately content-first: untracked files are in scope and the
only skips are VCS metadata, build or cache trees, the rendered book, and the
issue ledger (the fixed `SKIPPED_DIRECTORIES` list), plus the `@generated`
marker when a file declares it. Two real classes therefore gate by default:
generated TypeScript under tambor's `src/docs/content/generated/` (33 files)
and vendored agent-skill scripts under incitaciones' `.agents/` and
`.claude/skills/*/scripts/` (16 files), plus three more across miblioteca and
wanna. That is 52 gitignored sources outside the skip list fleet-wide.

Both dominant classes are already declared out of scope by the repositories
themselves: their `.gitignore` files cover these paths. Most codegen outputs
also lack the `@generated` marker, so the existing marker off-ramp does not
reach them. The only current remedy is a hand-written `sad.toml` `exclude`
glob per repository, and no init path seeds it, so every adopter rediscovers
the same list and the same generated directories.

## What Changes

- Discovery SHALL honor `.gitignore` files when the repository root holds a
  `.git` entry (a directory, or a file for a linked worktree or submodule).
  The ignore files SHALL be parsed directly: no git binary, no index, no
  user or global config, no `.git/info/exclude`, no network.
- Root and nested `.gitignore` files SHALL be read only where the walk
  reaches them. A deeper file SHALL override a shallower one, and the last
  matching rule within a file SHALL win.
- The grammar SHALL be closed: basename-at-any-depth for patterns without
  `/`, `/` anchoring at the ignore file's directory, trailing `/` for
  directory-only rules, `!` negation, `*` and `?` within one segment, `**`
  as a whole segment (trailing `/**` covers descendants only), and literal
  character classes, braces, and backslashes. CRLF line endings and leading
  or trailing blanks SHALL be trimmed.
- An ignored directory SHALL be pruned, so a negation cannot re-include a
  path below it, mirroring git.
- A missing, unreadable, or non-UTF-8 ignore file SHALL contribute no rules
  and SHALL NOT fail discovery.
- No counter, warning, or exemption count SHALL be added. Ignored paths are
  a pre-discovery scope rule, like `SKIPPED_DIRECTORIES`, not an exemption
  applied to an analyzed file, so the exemption list stays unchanged.
- The fixed skip list and `sad.toml` excludes SHALL keep running as they do;
  exclusion stays one-directional and there is no `sad.toml` re-include.

## Impact

- Spec: `openspec/specs/comment-gate/spec.md` gains one requirement,
  `Discovery honors gitignore inside a git work tree`, with six scenarios.
  No deployed requirement is modified.
- Code: `sad/src/check/discovery.rs` only. `sad/src/exemptions.rs` stays
  untouched: its matcher is root-anchored `sad.toml` exclude globbing, a
  different contract.
- Tests: new `sad/tests/check_discovery_gitignore_test.rs`. Existing
  discovery unit tests use bare temp trees, so they keep pinning the
  no-`.git` fallback.
- Docs: `docs/src/gate-model.md` gains an `Ignore files` subsection;
  `CHANGELOG.md` records the v0.2.0 behavior change.
- Ticket: `sad-z6q`; decision note `sad-8ah` option (a).

## Out of scope

- Reading the git index to keep tracked-but-ignored files gated. That is a
  documented divergence: the gate keys on tree content only.
- User, global, and `.git/info/exclude` sources. A verdict stays portable
  across machines.
- A per-path ignored counter or warning. A count over a scope decision the
  tree already records would change the envelope shape for no signal.
- Generated/vendor path-name heuristics and an init subcommand that seeds
  `sad.toml` excludes. Both guess intent the ignore file already states.