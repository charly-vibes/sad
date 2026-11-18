> *"It's sad (So sad), so sad*
> *It's a sad, sad situation*
> *And it's gettin' more and more absurd*
> *It's sad (So sad), so sad*
> *Why can't we talk it over?"*
> — Elton John

# Gate Model

> **Status: specified 2026-11-17 (design resolved 2026-11-16); v0.1
> implemented and dogfooded in this repository — pre-release, not yet on
> crates.io.** The EARS spec is deployed at
> [`openspec/specs/comment-gate/spec.md`](https://github.com/charly-vibes/sad/blob/main/openspec/specs/comment-gate/spec.md)
> (change archived 2026-11-18). The decision record and evidence ledger are
> [`design.md`](https://github.com/charly-vibes/sad/blob/main/openspec/changes/archive/2026-11-18-specify-gate-objective/design.md).
> This page records the v2 model.

## Premise

Code should be self-explanatory. Comments and doc strings are an apology for
code that failed to express itself — the fix is a refactor into simpler,
more expressive code, not more words.

sad's primary audience is **LLM coding agents**, which spray narration
("`# This function calculates the total..."`), apology comments
("`// HACK: this is confusing`"), and TODO stubs to look finished. Agents
respond to crisp mechanical rules, so sad is purist by design: anything
outside the channels below is a finding. No semantic judgment anywhere —
channels and counts only.

## Legitimate prose channels

Prose is allowed in exactly three places, all structurally checkable:

1. **File header** — a top-of-file module doc (`//!` in Rust, module
   docstring in Python, header block comment in TypeScript). Detection is
   keyword-based and order-free: all three of `Purpose`,
   `Responsibilities`, and `Rationale` must appear in the file's first
   block. `require_header` (see [Configuration](#configuration-sadtoml))
   gates the check; when it is on, a file missing any keyword yields
   `missing_header`. This is the one home for "why" prose.
2. **Doc channel** — every doc-comment (`///`, `/** */`) and docstring,
   regardless of visibility. The **summary lead**, the text up to the
   first blank line, is capped at `doc_summary_cap` rendered lines (see
   [Configuration](#configuration-sadtoml)). Everything after the first
   blank line is uncapped:
   Rust `# Errors` / `# Panics`, numpy `:param:`, TSDoc `@throws`. The cap
   is what stops agents laundering inline narration into doc-strings.
3. **Traceability markers** — a comment whose leading token is the
   configured `marker_prefix` followed by an ID matching the configured
   `marker_pattern` (both in [Configuration](#configuration-sadtoml)) that
   resolves against `.beads/issues.jsonl` or `openspec/specs/`. Each
   resolver no-ops while its source is absent, so a repo without beads
   never flags its markers; version-number-shaped text is prose unless
   prefixed. Doc-channel nodes never route to this channel, so a
   resolvable ID prefix cannot launder narration past the summary cap. An
   unresolvable ID is itself a finding — the anti-fabrication guardrail.
   Pure file lookup, no semantics.

**Exempt-but-observable principle:** every exempt channel's numbers — header
count, doc count, markers resolved, exemptions, suppression debt — appear
in the envelope. An agent cannot slowly fill an exempt channel without the
numbers moving.

## Exemptions

Licence/SPDX header blocks, `@generated` marker regions, and paths matching
`exclude` globs produce no findings. The envelope reports exemption counts,
plus the exempt regions a run skipped, so exemptions are as observable as
the channels.

## Suppression

Every suppression directive — `# noqa`, `# type: ignore`, `// @ts-ignore`,
`// eslint-disable-*`, and sad's own `sad:ignore <check-id> <reason>` — is
exempt-but-counted: never a prose finding, always a `suppression_debt`
entry (file, line, directive, check) in the envelope. A directive scopes
to exactly one check. When `suppression_cap` is set (see
[Configuration](#configuration-sadtoml)), a file whose debt exceeds it
emits `suppression_cap_exceeded`, so the gate cannot be routed by
ignore-spam with zero signal.

## Checks

| Check | Finding |
|---|---|
| `comment_outside_channel` | prose attributed to no channel |
| `commented_out_code` | prose block whose normalised body parses as code |
| `doc_summary_cap_exceeded` | doc summary lead over `doc_summary_cap` rendered lines |
| `unresolvable_marker` | marker-prefixed ID that resolves to nothing |
| `missing_header` | first block does not contain all three header keywords, when `require_header` is on |
| `suppression_cap_exceeded` | file over `suppression_cap` suppressions, when a cap is set |

`suppression_debt` is envelope data, never a finding.

## Commented-out code

`commented_out_code` is a finding kind beside `comment_outside_channel`,
never a seventh attribution class: every node keeps its channel. A
**block** is a maximal run of finding-eligible prose nodes with no row
gap (`next.line <= prev.line + prev.text.lines().count()`); same-row
nodes join and any other node — code, blank separator, doc, header,
marker, or suppression directive — splits. The block body is normalised
in a fixed order (line endings and BOM, per-kind marker and gutter
stripping, the newline join, common-prefix dedent, trailing-whitespace
and blank-end trim) and hashed as `path::symbol-path::content-hash`
with the anchor at the run's first node. One block is one finding.

The predicate parses the normalised body with the same pinned frontend
that attributed the nodes: a clean tree, non-empty root children, every
root child in the language's closed root-allow table, at least one shape
node, and no root-deny child, with root deny outranking shape. The
tables are transcribed against tree-sitter-rust 0.24,
tree-sitter-python 0.25, and tree-sitter-typescript 0.23; unknown kinds
fail closed to prose, so a grammar bump has one audit point. A predicate
miss or `[checks] commented_out_code = false` leaves the per-node
`comment_outside_channel` findings untouched — the class never buys
silence by guessing. A directive scoped to `commented_out_code` is
ordinary suppression debt and does not join a block.

The predicate input is bounded: a body within 8192 bytes and 512 lines
parses as one unit; a longer run is parsed in chunks cut at line
boundaries with the same bounds, and the block matches when any chunk
parses as a syntactic unit. The finding keeps the whole body, so chunking
never changes an ID. A construct straddling a chunk boundary fails closed
to prose, and a single line longer than the byte bound is treated as
prose.

Remediation is hardcoded: `git log -p --follow <path>` with "Delete the
commented-out code. If it is still needed, recover it from VCS history;
never restore it as a comment." Known false positive: a plain-comment
call example such as `// e.g. g();` parses as a call and fires; only doc
channels are excluded.

## Scope

Default mode analyzes only files whose governing revision — the most recent
commit touching the file — carries the configured attribution trailer (or
session tag; see [Configuration](#configuration-sadtoml)), plus files with
no commit yet. Only new/untracked files are
analyzed unconditionally: a tracked file with uncommitted edits follows
the governing-revision rule. `--all` analyzes every non-excluded file.
Attribution never comes from git heuristics — squash, rebase, and
multi-author files break inference, and a grandfathered baseline must not
mask new agent edits. The EARS spec's parenthetical still reads "new or
uncommitted"; it is being tightened to new/untracked.

The stats block makes the decision observable: `files_attributed` (kept by
the trailer), `files_without_commit` (kept as new work), and
`files_unattributed` (dropped). When a default run selects nothing but
candidates existed, text mode names the attribution scope and the
widening paths (`--all`, or commit with the trailer) in the same one-line
summary — a green gate that gated nothing never reads like a clean tree.

## Remediation directives

Each finding carries a hardcoded per-check template (not user-configurable)
as a genesis `RemediationEntry`. The templates encode the pressure system:
prose either *moves* to a legitimate channel, *dies*, or *the code
improves* (rename/extract until the comment is unnecessary). Templates
never say *which* refactor — naming and decomposition are the agent's job;
sad refuses prose as a substitute.

## Output contract (pretender mirror)

- `sad check` → genesis suite envelope (`Check` kind): findings list +
  `suppression_debt` + channel/exemption stats. Exit 0 on pass; 1 on
  findings or gate failure.
- **Stable finding IDs** — `<path>::<symbol-path>::<content-hash>`:
  survive line drift and unrelated edits; die only when the comment itself
  moves or changes. File-scoped findings (`missing_header`,
  `suppression_cap_exceeded`) use `-` as the symbol-path. Identical base
  identities in one run are discriminated by an occurrence suffix
  (`#2`, `#3`, …) in report order, so every ID is unique; the first
  occurrence keeps the bare identity.
- **`--adopt`** → writes `sad-baseline.json` of grandfathered IDs
  (committed). A baseline suppresses its IDs and is shrink-only: fixing
  prose makes the next run pass, while growing the baseline fails the run.
  Schema version 2 carries the occurrence-discriminated IDs; a version-1
  baseline still suppresses exact matches, and the adoption that rewrites
  it may add IDs exactly once (a legacy ID set cannot be reproduced).
- Single gate mode — boolean findings make gate+baseline equivalent to
  pretender's gate/tiered split; no warn tier.
- The same `sad check` invocation on the same tree produces the same
  envelope and exit code as a pre-hook and in CI. Operationally, the commit
  boundary still moves scope: the pre-hook sees the working tree, so
  new/untracked files are gated there, while CI gates committed revisions.

## Configuration (`sad.toml`)

| Knob | Default | Effect |
|---|---|---|
| `checks` | all enabled | per-check toggles |
| `doc_summary_cap` | `1` | rendered lines allowed in a doc summary lead |
| `marker_prefix` | `see` | leading token for a traceability marker |
| `marker_pattern` | `\b[a-z][a-z0-9]*-[a-z0-9]+\b` | ID shape after the prefix |
| `resolvers` | `.beads/issues.jsonl`, `openspec/specs/` | resolution sources; each no-ops while absent |
| `require_header` | on only in sad's own repo | enable `missing_header` |
| `suppression_cap` | off (per analyzed file) | maximum suppression debt per file |
| `exclude` | none | globs whose paths produce no findings |
| `attribution.trailer` | `Assisted-by` | commit trailer (or session tag) marking agent-attributed commits |

## Languages

Rust, Python, TypeScript, and JavaScript — v1 scope, tree-sitter
comment-node extraction with channel attribution, grammar pins inherited
from pretender's known-good set. The gated extension set is exactly
`.rs`, `.py`, `.ts`, `.js`, and `.mjs`; JavaScript files are parsed with
the TypeScript frontend. Adjacent extensions the gate cannot parse
(`.tsx`, `.mts`, `.cts`, `.jsx`) are counted in `files_unsupported` and
named by one `unsupported_extension` envelope warning each, never dropped
silently; JSX/TSX parsing is a separate decision. Rust ships first so sad
gates itself.

Selected sources that cannot be read as UTF-8 are counted in
`files_skipped` and named by one `unreadable_source` envelope warning each,
carrying the path and a stable reason (`not valid UTF-8`, `permission
denied`, `file disappeared during the run`, or the operating-system error
for anything else). The gate never guesses an encoding and invents no
findings for skipped content; the warnings keep the skip visible but do
not by themselves fail the run. Text mode appends the same paths and
reasons, so a passing line never hides a source it did not judge.

## Boundary

[pretender](https://github.com/charly-vibes/pretender) gates structure
(complexity, duplication, mutation), sad gates prose,
[vampiro](https://github.com/charly-vibes/vampiro) gates composition. No
tool reads another's input; a complex function plus an explanatory comment
is intentional stacking — both fire, same fix.

## Determinism

No network, no models, no randomness. Same input tree plus the same repo
ledgers (`.beads/issues.jsonl`, `openspec/specs/`) → same findings. Marker
resolution is file lookup only.

## Evidence status

- **Evidence-backed:** suppression debt and its envelope;
  unresolvable-marker detection; the summary-lead cap.
- **Design-inherited:** the three-channel structure — components have
  support, the architecture does not.
- **Bets:** the header requirement (zero corpus support; presence mandates
  tend toward stub noise) and the premise that banning prose improves code
  (correlational only).

The full ledger is in
[`design.md`](https://github.com/charly-vibes/sad/blob/main/openspec/changes/archive/2026-11-18-specify-gate-objective/design.md).

## Resolved decisions

Nothing on this page is open. The knobs deferred by the v1 draft are
resolved in the sections above: doc cap (Legitimate prose channels),
header strictness and default (same section), generated/vendor excludes
(Exemptions), and grammar pins (Languages). Rejected alternatives live in
`design.md`; changing any decision requires a new spec delta.
