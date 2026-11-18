# Gitignore and generated-path off-ramp

Status: decision note for `sad-8ah`. Chooses option (a): honor `.gitignore` inside a
git work tree. No code, spec, or docs change here; the follow-up implementation ticket
is `sad-z6q`, and the deployed spec needs an OpenSpec delta before that lands.

## Context and field counts

Discovery is deliberately content-first: untracked files are in scope, the only skips
are VCS metadata, build/cache trees, the rendered book, and the issue ledger (the
fixed 14-name `SKIPPED_DIRECTORIES` list), plus the `@generated` marker when a file
declares it. Two real classes therefore gate by default:

| Repo | Class | Count |
|---|---|---|
| tambor | generated `.ts` under `src/docs/content/generated/` | 33 |
| incitaciones | vendored agent-skill scripts under `.agents/` \| `.claude/skills/*/scripts/` | 16 |
| miblioteca | generated/vendored sources | 2 |
| wanna | generated/vendored source | 1 |
| **Fleet** | gitignored sources outside the skip list | **52** |

Both dominant classes are already declared out of scope by the repositories
themselves: their `.gitignore` files cover these paths (tambor via
`src/docs/content/generated/`, incitaciones via its skill trees). Most codegen
outputs also lack the `@generated` marker, so the existing marker off-ramp does not
reach them. The only current remedy is a hand-written `sad.toml` `exclude` glob per
repo, and no init path seeds it — every adopter rediscovers the same 14-name list and
the same generated directories.

## Decision

Honor `.gitignore` files during source discovery, inside a git work tree, by parsing
the ignore files directly. No git binary, no index, no user/global config, no
`.git/info/exclude`, no network. Ignore files are tree content, so the walk stays a
pure function of the tree — the property the current discovery module is built around.

This is the only option that fixes both field classes with zero per-repo
configuration: the author has already recorded intent in the tree, and sad reads it
instead of asking for a second declaration in `sad.toml`.

### Trigger and scope

- Filtering applies only when `<repo>/.git` exists — a directory, or a file for a
  linked worktree or submodule. The `.git` contents are never read, and there is no
  upward search from the repo root passed to discovery.
- Root and nested `.gitignore` files are read. Only ignore files on directories the
  walk actually reaches contribute rules; a `.gitignore` inside a skipped tree is
  never read (same as the existing walk).
- The fixed skip list still runs first and is unchanged. `.git` remains skipped.
- `sad.toml` excludes still run after discovery, unchanged. There is no re-include
  override: exclusion remains one-directional.

### Determinism caveat (accepted)

The file list remains a deterministic function of tree bytes plus repo-relative path:
no clock, network, environment, HOME, global excludes, index, or git invocation. But
ignore files are part of the tree, and the tree is not frozen: a branch switch,
`git clean`, partial checkout, or a nested `.gitignore` added by a dependency changes
which sources are gated even when the source bytes do not change. This is the
accepted price of zero configuration and must be stated in the spec and the docs:
`git check-ignore -v <path>` explains every skip, so the behavior is auditable.

Two divergences from `git status --ignored` are deliberate and documented:

1. **Tracked status is not consulted.** A file that matches an ignore pattern is
   skipped even if it is tracked (e.g. committed with `git add -f`), because the index
   is not read. Git keeps tracking such files; sad does not gate them.
2. **Only tree-local ignore files are read.** `core.excludesFile`, `$XDG_CONFIG_HOME`,
   and `.git/info/exclude` are ignored, so a path ignored on one machine by user
   config is still gated everywhere. This keeps a verdict portable across machines.

Fallbacks are total, never fatal: a missing, unreadable, or non-UTF-8 root `.gitignore`
means no ignore filtering at all — the exact pre-change walk; an unreadable nested file
contributes no rules from that file, so shallower rules still apply; and a tree without
`.git` never filters. A malformed pattern cannot error either — the grammar is closed,
so every character outside `*`, `?`, `**`, `!`, and a trailing `/` is literal. The worst
case of the feature is therefore the status quo.

### Why not (b): generated/vendor path heuristics

A vocabulary like `generated`, `vendor`, `dist`, `.agents`, `.claude` is unbounded and
repo-specific: it cannot separate tambor's `src/docs/content/generated/` from a
hand-written directory someone happens to name `generated`, and incitaciones'
`.agents/skills/*/scripts/` is a vendor convention knowable only from that repo's
configuration. Matching on names would silently skip legitimate source in other
trees — the opposite of a gate — and would still need per-repo additions, i.e. option
(c) by another name, without the author's declared intent. Ignore files are exactly
that declaration, already versioned.

### Why not (c): an init subcommand that seeds `sad.toml` excludes

The subcommand would have to detect the same paths, so it inherits (b)'s ambiguity or
hard-codes the same guesses. It also writes a second, stateful artifact that goes
stale as generated directories move, must be re-run in every adopting repo, and adds
a config-review step to the gate's own dogfood config. The tree already states the
fact; copying it into `sad.toml` creates drift for no gain. (Extending `sad init` for
fleet wiring remains reasonable for unrelated settings.)

## Semantics

Rules are evaluated per directory entry during the walk, before extension filtering.
Precedence: shallower ignore files first, deeper files later; within one file,
top-to-bottom; the last matching rule wins. An ignored directory is pruned, so a
negation cannot re-include a path below an ignored directory (this mirrors git).

| Ignore-file form | Supported | Semantics |
|---|---|---|
| blank line, `# comment` | yes | no rule; a leading `#` is always a comment (no `\#` escape) |
| `name` (no `/`, no `**`) | yes | compiled as `**/name`: matches a file or directory named `name` at any depth; an ignored directory prunes its subtree |
| `*.ts`, `pre*`, `a?c` | yes | basename match at any depth; `*` matches any run including empty, `?` exactly one character, neither crosses `/` |
| `path/to/name` (embedded `/`) | yes | anchored at the ignore file's directory; the entry's path relative to that directory is matched from the start |
| `/name` | yes | same as `name` anchored at the ignore file's directory; the leading `/` only anchors and is not a segment |
| `name/` (trailing `/`) | yes | directory-only: matches a directory named `name` (any depth when it has no other `/`); a regular file named `name` is untouched |
| `dir/**`, `**/name`, `a/**/b` | yes | `**` is a segment matching zero or more whole segments (`**/name` matches `name` at the root and at any depth); unlike `exemptions::glob_match`, a trailing `/**` matches only descendants, never `dir` itself, so a nested `.gitignore` inside `dir` can still negate — this mirrors git |
| `!pattern` | yes | negation: re-includes an entry when it is the last matching rule and no ancestor directory is ignored; a literal leading `!` is not expressible |
| `[a-z]`, `{a,b}`, `\` escapes, trailing-space escapes | no | not interpreted; those characters are literal (same closed grammar as `exemptions::glob_match`). None of the four field repos' `.gitignore` files uses them; they do use `!` negation, which is supported |
| `**` inside a longer segment (`a**b`) | no | behaves like `*`, mirroring `exemptions::glob_match` |
| `.git/info/exclude`, global excludes, `.gitignore` outside the tree | no | never read |
| CRLF, leading/trailing blanks | yes | trailing `\r`, spaces, and tabs are trimmed per line |

Matching mirrors `exemptions::glob_match`: normalization is forward-slash,
repo-relative, no empty or `.` segments; `*`/`?` never cross a separator; `**` is a
whole segment; character classes and brace expansion do not exist. The two anchoring
differences — no-slash patterns match at any depth, and a rule is relative to its own
directory — come from gitignore, not from the exclude-glob contract, which is why the
implementation stays in `discovery.rs` and does not reuse the private matcher in
`exemptions.rs` (that would have to grow an anchoring parameter for a different
contract).

### Envelope contract

Ignored paths are outside the discovered candidate set, like `SKIPPED_DIRECTORIES`:
they add no `files_ignored` counter, no warning, and no exemption count. The
"exempt-but-observable" principle covers exemptions applied *to analyzed files*
(licence/SPDX, `@generated`, `sad.toml` excludes); gitignore is a pre-discovery scope
rule, and the spec delta must state that explicitly so the exemption list is not read
as closed over the new behavior. The alternative — a per-path ignored count — would
change the envelope shape and output contract for a scope decision the tree already
records, and is not needed to fix the field problem.

## Migration note

After upgrade, gitignored sources stop being gated fleet-wide: tambor's 33 generated
`.ts`, incitaciones' 16 vendored scripts, and the remaining 3. No repo needs a config
change; no baseline rewrite is required (the disappeared findings simply stop being
evaluated, and their old IDs remain harmless). Candidate and selected-file envelope
counts drop accordingly.

If a repository *wants* such sources gated, the move is in the tree: remove or narrow
the ignore rule, add a `!` re-include in a deeper `.gitignore`, or move the file out of
the ignored directory. `sad.toml` cannot re-include; it only narrows further. For
generated code that is tracked and reviewed, keep it unignored (and consider the
`@generated` marker, whose exemption remains counted). Re-inclusion below a
pruned directory is not possible — same as git.

## Follow-up implementation (`sad-z6q`)

Target files: `sad/src/check/discovery.rs`,
`sad/tests/check_discovery_gitignore_test.rs` (new), and
`openspec/specs/comment-gate/spec.md` through a new OpenSpec change (never a direct
deployed-spec edit).

Spec delta: ADD one requirement, `Discovery honors gitignore inside a git work tree`,
with scenarios `gitignored_sources_are_not_gated` and
`not_gated_source_in_non_ignored_path_stays_gated`; its prose carries the contract
above (trigger, precedence, grammar, fallbacks, tracked-status divergence,
no-counter scope rule). No deployed requirement is modified: the exemption list stays
the three analyzed-file channels.

Red phase — these test functions must exist and fail before implementation, then pass:

| Test | Fixture | Expectation |
|---|---|---|
| `gitignored_sources_are_not_gated` | temp tree with `.git/`, root `.gitignore` covering `src/generated/`, and `src/generated/api.ts` containing a finding-eligible prose comment | `discover_files` omits `src/generated/api.ts`; a `sad check --all` run emits no finding for it |
| `not_gated_source_in_non_ignored_path_stays_gated` | same tree with the same-looking path but no matching ignore rule (and no `.gitignore` entry), prose present | the file is discovered and its prose yields `comment_outside_channel`, proving rule-content matching, not name heuristics |

The existing discovery unit tests already cover the no-`.git` fallback (they use bare
temp trees), so no third red test is named; the new test file must also pin `!`
negation (used by miblioteca) and trailing-`/` directory-only pruning.

Implementation sketch: detect `.git` at the root; maintain the per-directory ignore
stack while walking; compile a rule into `(anchored, directory_only, negated,
segments)`; prune ignored directories; never read `.git` internals.