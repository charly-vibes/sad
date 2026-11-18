# Design: gitignore-aware-discovery

## Channel: a pre-discovery scope rule, not an exemption

Ignored paths leave the candidate set before analysis, exactly like
`SKIPPED_DIRECTORIES`. The "exempt-but-observable" principle covers channels
applied *to analyzed files* — licence/SPDX headers, `@generated`, `sad.toml`
excludes — and those keep their counters. Gitignore is upstream of all of
them: the file is never discovered, so there is no finding to exempt and no
number to move. Adding `files_ignored` would change the envelope shape and
output contract for a decision the tree already records, so the exemption
list stays the three analyzed-file channels.

## Why parse the ignore files instead of asking git

A `git check-ignore` subprocess would introduce a binary dependency, an index
read, `$HOME` and global config, `.git/info/exclude`, and a failure mode when
git is absent. Parsing the files keeps the walk a pure function of tree bytes
plus repo-relative path: no clock, network, environment, index, or
invocation. `git check-ignore -v <path>` remains the audit tool that explains
every skip after the fact.

## Trigger and scope

Filtering applies only when `<repo>/.git` exists, as a directory or as a file
for a linked worktree or submodule. The `.git` contents are never read, and
there is no upward search from the repository root passed to discovery, so a
directory inside a larger work tree is filtered according to its own root
only. A tree without `.git` keeps the exact pre-change walk.

Root and nested `.gitignore` files are read while walking. A file is read
only when the walk reaches its directory: a `.gitignore` inside a skipped or
pruned tree never contributes rules. The fixed skip list still runs first and
is unchanged, `.git` included.

Precedence is shallower-first, deeper-later, and top-to-bottom within one
file; the last matching rule wins. A negative rule therefore overrides an
earlier ignore rule from the same or a shallower file, but an ignored
directory is pruned before its children are visited, so a negation cannot
re-include a path below an ignored directory. That mirrors git and keeps the
pruning cost down.

## Matcher

Rules compile into `(anchored, directory_only, negated, segments)` and are
evaluated per directory entry relative to the ignore file's directory:

| Form | Semantics |
|---|---|
| blank line, `# comment` | no rule; a leading `#` is always a comment |
| `name` | `**/name`: a file or directory named `name` at any depth |
| `*.ts`, `pre*`, `a?c` | basename at any depth; `*` any run including empty, `?` one character, neither crosses `/` |
| `path/to/name` | anchored at the ignore file's directory, matched from the start |
| `/name` | anchored at the ignore file's directory; the leading `/` only anchors |
| `name/` | directory-only; a regular file named `name` is untouched |
| `dir/**`, `**/name`, `a/**/b` | `**` is a whole segment spanning zero or more segments; a trailing `/**` matches descendants only, never `dir` itself |
| `!pattern` | negation; a literal leading `!` is not expressible |
| `[a-z]`, `{a,b}`, `\` escapes | literal; the grammar is closed |
| `a**b` | behaves like `*`, mirroring `exemptions::glob_match` |

Matching normalizes to forward slashes with no empty or `.` segments. The
implementation lives in `discovery.rs` and does not reuse the private matcher
in `exemptions.rs`: that matcher is root-anchored for `sad.toml` excludes,
while gitignore needs per-file anchoring and basename-at-any-depth, so
sharing it would mean growing an anchoring parameter for a different
contract.

## Fallbacks are total

A missing, unreadable, or non-UTF-8 root `.gitignore` means no ignore
filtering — the pre-change walk. An unreadable nested file contributes no
rules from that file, so shallower rules still apply. A tree without `.git`
never filters. A malformed pattern cannot error either, because every
character outside `*`, `?`, `**`, `!`, and a trailing `/` is literal. The
worst case of the feature is the status quo.

## Deliberate divergences from `git status --ignored`

1. **Tracked status is not consulted.** A path matching an ignore pattern is
   skipped even if it is tracked (for example committed with `git add -f`),
   because the index is never read. Git keeps tracking such a file; sad does
   not gate it. Repositories that want tracked generated code gated keep it
   unignored and optionally mark it `@generated`, whose exemption is counted.
2. **Only tree-local ignore files are read.** `core.excludesFile`,
   `$XDG_CONFIG_HOME`, and `.git/info/exclude` are ignored, so a path ignored
   on one machine by user config is still gated everywhere.

## Determinism caveat

The discovered set remains a deterministic function of tree bytes plus
repo-relative path, but ignore files are tree content and the tree is not
frozen: a branch switch, `git clean`, a partial checkout, or a nested
`.gitignore` added by a dependency can change which sources are gated even
when the source bytes do not change. This is the accepted price of zero
configuration and is stated in the spec and the docs.

## Re-inclusion is a tree edit

There is no `sad.toml` re-include: exclusion remains one-directional. To gate
a path again, remove or narrow the ignore rule, add a `!` re-include in a
deeper `.gitignore`, or move the file out of the ignored directory.
Re-inclusion below a pruned directory is impossible, same as git.

## Rejected alternatives

- **Generated/vendor path heuristics.** A vocabulary like `generated`,
  `vendor`, `dist`, `.agents` cannot separate tambor's generated directory
  from a hand-written directory someone names `generated`, and would still
  need per-repo additions.
- **An init subcommand that seeds `sad.toml` excludes.** It has to detect the
  same paths, writes a second artifact that goes stale as directories move,
  and needs re-running in every repository.
- **A skip counter or warning per ignored path.** Cost proportional to tree
  content for a scope decision, and it would misclassify ignore filtering as
  an exemption.