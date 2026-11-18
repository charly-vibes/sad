> *"It's sad (So sad), so sad*
> *It's a sad, sad situation*
> *And it's gettin' more and more absurd*
> *It's sad (So sad), so sad*
> *Why can't we talk it over?"*
> — Elton John

# Gate Model

> **Status: specified 2026-11-17 (design resolved 2026-11-16); v0.2.0
> implemented and dogfooded in this repository — pre-release, not yet on
> crates.io.** The EARS spec is deployed at
> [`openspec/specs/comment-gate/spec.md`](https://github.com/charly-vibes/sad/blob/main/openspec/specs/comment-gate/spec.md)
> (change archived 2026-11-18). The decision record and evidence ledger are
> [`design.md`](https://github.com/charly-vibes/sad/blob/main/openspec/changes/archive/2026-11-18-specify-gate-objective/design.md);
> the triage design and its corpus evidence live in
> `openspec/changes/archive/2026-11-18-comment-smell-triage/`. This page records the v2 model
> plus the triage pass.

## Premise

Code should be self-explanatory. Comments and doc strings are an apology for
code that failed to express itself — the fix is a refactor into simpler,
more expressive code, not more words.

sad's primary audience is **LLM coding agents**, which spray narration
("`# This function calculates the total..."`), apology comments
("`// HACK: this is confusing`"), and TODO stubs to look finished. Agents
respond to crisp mechanical rules, so sad stays purist about what it
records: every prose node outside the channels below is still a finding,
and the channels never become an escape hatch. Enforcement is narrower — a
run blocks only on the baseline delta, and only a confident smell is an
error. No semantic judgment anywhere: structural attribution plus
deterministic smell rules, never a model.

## Legitimate prose channels

Prose is allowed in exactly four places, all structurally checkable:

1. **File header** — a top-of-file module doc (`//!` in Rust, module
   docstring in Python, header block comment in TypeScript). Detection is
   keyword-based and order-free: all three of `Purpose`,
   `Responsibilities`, and `Rationale` must appear in the file's first
   block. `require_header` (see [Configuration](#configuration-sadtoml))
   gates the check; when it is on, a file missing any keyword yields
   `missing_header`. This is the first home for "why" prose.
2. **Doc channel** — every doc-comment (`///`, `/** */`) and docstring,
   regardless of visibility. The **summary lead**, the text up to the
   first blank line, is capped at `doc_summary_cap` sentences (see
   [Configuration](#configuration-sadtoml)), not rendered lines: one
   sentence passes however many source lines it wraps. Inline code spans
   and URLs are stripped before counting, and a boundary is `.`, `!`, or
   `?` followed by whitespace and an uppercase character, digit, a quote,
   or the end of the lead — so `e.g.`, `i.e.`, `etc.`, `vs.`, `cf.`,
   `approx.`, versions like `v0.1`, and `Node.js` do not split a
   sentence. Everything after the first blank line is uncapped and not
   counted: Rust `# Errors` / `# Panics`, numpy `:param:`, TSDoc
   `@throws`. The cap is what stops agents laundering inline narration
   into doc-strings.
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
4. **Rationale** — a comment whose body begins with one of the closed tag
   set `SAFETY:`, `INVARIANT:`, `SECURITY:`, or `WHY:` — uppercase, then a
   colon, then a non-empty body. It is counted in
   `stats.comments_rationale` and is not debt: this is the one inline home
   for knowledge that must survive beside the code (Rust's `// SAFETY:`
   convention is the model). `NOTE:` stays prose, because it usually
   labels narration, and the tag set never grows without a spec change.
   Pure leading-token test, no semantics.

Attribution precedence is structural and fixed: **file header > doc >
marker > rationale > suppression > exemption > prose**. So `//! SAFETY:`
is a header, `/// WHY:` is doc, and `// see <id> SAFETY: ...` is a marker;
only a plain comment that falls through the earlier classes can be
rationale, and only a comment that falls through every class is
finding-eligible prose. The exemption class includes Python file metadata:
a `#!` shebang on source line 1 and a PEP 263 coding cookie
(`-*- coding: utf-8 -*-`, `coding: utf-8`, `coding=utf-8`) on source lines
1–2 are exempt-but-observable, counted by the exemption summary and never
prose; the same text on a later line stays finding-eligible prose.

**Exempt-but-observable principle:** every exempt channel's numbers —
header count, doc count, markers resolved, rationale count, exemptions,
suppression debt — appear in the envelope. An agent cannot slowly fill an
exempt channel without the numbers moving.

## Exemptions

Licence/SPDX header blocks, `@generated` marker regions, paths matching
`exclude` globs, and Python file metadata produce no findings. Licence text
beside an SPDX tag belongs to the licence block when the body opens with a
bounded marker: `Copyright` followed by a four-digit year, `(c)`, or `©`; a
leading `©` or `(c)`; `Licensed under`; `License:`; or
`All rights reserved`. The bound is the marker shape, so prose that merely
names copyright or a licence — `Copyright is a legal concept`,
`License negotiation happens elsewhere` — stays finding-eligible. Python
file metadata is bounded by position: a shebang only on source line 1, a
coding cookie only on lines 1–2, and the same text later stays prose. The
envelope reports exemption counts, plus the exempt regions a run skipped,
so exemptions are as observable as the channels.

## Suppression

Every suppression directive — `# noqa`, `# type: ignore`, `// @ts-ignore`,
`// @ts-expect-error`, `// eslint-disable-*`, and sad's own
`sad:ignore <check-id> <reason>` — is exempt-but-counted: never a prose
finding, always a `suppression_debt` entry (file, line, directive, check)
in the envelope. A directive scopes to a single check and withholds no
other finding; the two `@ts-*` forms are unscoped and serialise as
`ts_ignore` and `ts_expect_error`. When `suppression_cap` is set (see
[Configuration](#configuration-sadtoml)), a file whose debt exceeds it
emits `suppression_cap_exceeded`, so the gate cannot be routed by
ignore-spam with zero signal.

## Checks

| Check | Finding |
|---|---|
| `comment_outside_channel` | prose attributed to no channel |
| `commented_out_code` | prose block whose normalised body is code-shaped |
| `doc_summary_cap_exceeded` | doc summary lead over `doc_summary_cap` sentences |
| `unresolvable_marker` | marker-prefixed ID that resolves to nothing |
| `missing_header` | first block does not contain all three header keywords, when `require_header` is on |
| `suppression_cap_exceeded` | file over `suppression_cap` suppressions, when a cap is set |

`suppression_debt` is envelope data, never a finding. Every check's
`smell` and severity are in [Severity and smells](#severity-and-smells).

## Severity and smells

A comment is a diagnostic trigger, not a verdict. Every finding carries
`severity` (`error` or `warning`), and `smell` is nullable — present only
for findings derived from comment prose, serialized as snake_case strings
with both fields always present.

| Finding | `smell` | Severity |
|---|---|---|
| `comment_outside_channel` (narration, apology, todo) | that smell | `error` |
| `comment_outside_channel` (banner, unclassified) | that smell | `warning` |
| `commented_out_code` | `commented_out_code` | `error` |
| `doc_summary_cap_exceeded` | `doc_cap` | `warning` |
| `missing_header` | `null` | `error` |
| `unresolvable_marker` | `null` | `error` |
| `suppression_cap_exceeded` | `null` | `error` |

Classification is one post-pass after every check, over each **maximal
adjacent prose run**, so a sentence split across three `//` nodes is
judged as one unit. Every member node keeps its own ID and receives the
run's smell and severity; narration scope covers function bodies,
module-level statements, and class bodies. The rules are deterministic:

| Smell | Rule |
|---|---|
| `todo` | body matches `TODO`, `FIXME`, or `XXX` as a whole word, and no resolvable marker is in the node |
| `apology` | body matches `for now`, `temporary fix` / `temporary hack` / `temporary workaround` / `temporary shim` / `temporary patch`, `workaround`, `hack:`, `legacy shim`, or `to be removed`, and no resolvable marker is in the node; bare `temporary` or `hack` is not enough |
| `narration` | the run shares at least two distinctive tokens with the next non-comment statement in the same scope, or starts with a sequencer and names the next call (see below) |
| `banner` | body has no alphanumerics, or is a run of three or more repeated separator characters; applies to `commented_out_code` bodies too |
| `unclassified` | prose matching none of the above |

**Narration.** Take the run and the first non-empty, non-comment source
line after it in the same scope. Tokenize both lowercased, keeping
identifiers and literals of length at least 4 and dropping a fixed
English/stopword/keyword list (`this`, `that`, `with`, `from`, `then`,
`when`, `where`, `which`, `will`, `have`, `into`, `over`, `under`,
`about`, `after`, `before`, `these`, `those`, `their`, `there`,
`function`, `value`, `return`, `create`, `make`, `call`, `line`, plus
language keywords). Tokens match by prefix, so `parse` meets
`parse_config`. The run is narration when at least two distinct comment
tokens appear in that next statement, or when the run starts with
`now`, `then`, `first`, `next`, or `finally` and the statement contains a
call or identifier named in the run. With no next statement, or no scope,
the run is `unclassified`.

**Ambiguity yields `unclassified`** — a warning. The classifier is biased
to false negatives: false-positive errors are not acceptable, so only a
confident smell blocks.

## Commented-out code

`commented_out_code` is a finding kind beside `comment_outside_channel`,
never an attribution class of its own: every node keeps its channel. A
**block** is a maximal run of finding-eligible prose nodes with no row
gap (`next.line <= prev.line + prev.text.lines().count()`); same-row
nodes join and any other node — code, blank separator, doc, header,
marker, or suppression directive — splits. The block body is normalised
in a fixed order (line endings and BOM, per-kind marker and gutter
stripping, the newline join, common-prefix dedent, trailing-whitespace
and blank-end trim) and hashed as `path::symbol-path::content-hash`
with the anchor at the run's first node. One block is one finding,
carrying `smell: commented_out_code` and error severity; a block
reclassified as `banner` stays a warning.

The predicate parses the normalised body with the same pinned frontend
that attributed the nodes, and a body qualifies only on a clean tree whose
root children are non-empty, every root child is in the language's closed
root-allow table, no root-deny child is present, and at least one root
child is a declaration or a control statement from the closed
per-language code-shape table; root deny outranks shape. A body with no
alphanumeric character is rejected before the parser, so a ruler or
divider never reaches the class. A root made only of bare
expression-statement payloads — a call, an assignment, a new-expression, a
formula, an annotation, `// e.g. g();` — carries no declaration or control
child and stays prose. The tables are transcribed against
tree-sitter-rust 0.24, tree-sitter-python 0.25, and
tree-sitter-typescript 0.23; unknown kinds fail closed to prose, so a
grammar bump has one audit point. A predicate miss or
`[checks] commented_out_code = false` leaves the per-node
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
never restore it as a comment." Call-shaped examples such as
`// e.g. g();` stay prose under the code-shape rule, and doc channels are
excluded before the predicate runs.

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

Each finding carries a hardcoded per-smell template (not user-configurable)
as a genesis `RemediationEntry`. The templates encode the pressure system:
prose either *moves* to a legitimate channel, *dies*, or *the code
improves* (rename/extract until the comment is unnecessary).

| Smell | Acceptable outcomes |
|---|---|
| `narration` | rename or extract until the code says it (preferred), or delete when the comment is redundant |
| `apology`, `todo` | cite a resolvable ID — which moves the node to the marker channel — or fix the problem |
| `commented_out_code` | delete and recover from VCS history |
| `banner` | delete |
| `unclassified`, `doc_cap` | move to header/doc with a one-sentence summary, tag as rationale, or cite a marker; deletion is offered only for a redundant comment |

The contract is **"do not delete knowledge to satisfy the gate."**
Templates never say *which* refactor — naming and decomposition are the
agent's job; sad refuses prose as a substitute.

## Output contract (pretender mirror)

- `sad check` → genesis suite envelope (`Check` kind): findings (each with
  `severity` and nullable `smell`) + `suppression_debt` + channel/exemption
  stats. `stats.findings` counts every new finding after baseline
  suppression; `stats.findings_blocking` counts the new errors. `ok` is
  true iff `findings_blocking == 0`, and the exit status mirrors `ok`
  (0 pass, 1 fail).
- **Warnings report; errors block.** A default run lists warnings in the
  envelope and text render and still passes. Enforcement is the baseline
  delta: an adopted id stays grandfathered whatever its severity, so a
  whole-tree run without a baseline reports every finding and fails only
  on errors.
- **`--strict`** restores the any-finding failure: `ok` is true iff
  `findings == 0` (exit status mirrors it), while `findings_blocking`
  still counts errors only. Hooks and CI configs written against v0.1's
  "any finding fails" adopt `--strict` to keep their behaviour.
- **Stable finding IDs** — `<path>::<symbol-path>::<content-hash>`:
  survive line drift and unrelated edits; die only when the comment itself
  moves or changes. File-scoped findings (`missing_header`,
  `suppression_cap_exceeded`) use `-` as the symbol-path. Identical base
  identities in one run are discriminated by an occurrence suffix
  (`#2`, `#3`, …) in report order, so every ID is unique; the first
  occurrence keeps the bare identity. IDs never encode severity, so a
  classifier change that promotes an adopted warning to error stays
  grandfathered until re-adoption.
- **`--adopt`** → writes `sad-baseline.json` of grandfathered IDs
  (committed). Adoption covers warnings as well as errors; a baseline
  suppresses its IDs and is shrink-only: fixing prose makes the next run
  pass, while growing the baseline fails the run.
  Schema version 2 carries the occurrence-discriminated IDs; a version-1
  baseline still suppresses exact matches, and the adoption that rewrites
  it may add IDs exactly once (a legacy ID set cannot be reproduced).
- Two severity tiers map the old single gate onto pretender's
  gate/tiered split: errors are the gate, warnings the tier.
- Text mode, one line:
  - nothing found: `sad check — pass: 0 findings in M files`
  - warnings only: `sad check — pass: 0 blocking, W warnings in M files`
  - errors: `sad check — fail: B blocking, W warnings in M files; <first>`
- The same `sad check` invocation on the same tree produces the same
  envelope and exit code as a pre-hook and in CI. Operationally, the commit
  boundary still moves scope: the pre-hook sees the working tree, so
  new/untracked files are gated there, while CI gates committed revisions.

## Configuration (`sad.toml`)

| Knob | Default | Effect |
|---|---|---|
| `checks` | all enabled | per-check toggles |
| `doc_summary_cap` | `1` | sentences allowed in the doc summary lead |
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
`.rs`, `.py`, `.pyi`, `.ts`, `.js`, and `.mjs`; `.pyi` stubs are parsed as
Python with the unchanged doc channel (module, class, and function
docstrings) and no stub-specific rule, and JavaScript files are parsed
with the TypeScript frontend. Adjacent extensions the gate cannot parse
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

### Skipped directories

Discovery never traverses 21 directory names: `.git`, `.beads`,
`node_modules`, `vendor`, `third_party`, `.venv`, `venv`, `.tox`,
`.mypy_cache`, `.ruff_cache`, `.pytest_cache`, `__pycache__`, `target`,
`dist`, `build`, `out`, `coverage`, `.next`, `.turbo`, `.svelte-kit`, and
`book`. The names are reserved for build, vendor, and cache trees and
match at any depth. Five are also plausible source-directory names —
`target`, `build`, `dist`, `book`, `out` — so a nested occurrence at depth
two or deeper produces one `skipped_source_directory` envelope warning
naming the directory (`src/target/`), while a root-level build tree stays
silent. Files inside a skipped directory yield no findings and are never
enumerated; text mode names each skipped directory, and the warnings never
change the exit status.

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
  unresolvable-marker detection; the summary-lead cap. The diagnosis
  behind the triage model is recorded in
  `openspec/changes/archive/2026-11-18-comment-smell-triage/evidence.md`: 48
  repositories, 3,873 files, 58,781 findings, of which 42.5% were in tests
  and 24.7% mechanical noise, while TODO comments were 0.03% and
  apology/workaround phrases 0.04%. The same corpus showed 39.6% of
  `doc_summary_cap_exceeded` findings were single sentences wrapped across
  source lines, and every inspected `commented_out_code` finding was a
  banner, formula, or expected-value annotation rather than dead code.
- **Design-inherited:** the four-channel structure and the smell rules —
  components have support, the architecture does not. The classifier is
  conservative by construction, so the sampled share of genuine narration
  (roughly 10-15% of the non-test inline sample) is expected to remain a
  minority signal.
- **Bets:** the header requirement (zero corpus support; presence mandates
  tend toward stub noise), the premise that banning prose improves code
  (correlational only), and that the two-tier verdict — warnings report,
  errors block, `--strict` restores the single tier — keeps the gate honest
  while the heuristics mature.

The v1 ledger is in
[`design.md`](https://github.com/charly-vibes/sad/blob/main/openspec/changes/archive/2026-11-18-specify-gate-objective/design.md);
the triage evidence is `openspec/changes/archive/2026-11-18-comment-smell-triage/evidence.md`.

## Resolved decisions

Nothing on this page is open. The knobs deferred by the v1 draft are
resolved in the sections above: doc cap (Legitimate prose channels),
header strictness and default (same section), generated/vendor excludes
(Exemptions), and grammar pins (Languages). The triage model is resolved
too: severity on every finding with nullable `smell` ([Severity and
smells](#severity-and-smells)), the closed rationale tag set and its
place in the structural precedence (Legitimate prose channels), the
sentence-counted doc cap (same section), per-smell remediation
dispositions, and the warning-tolerant verdict with `--strict`
([Output contract](#output-contract-pretender-mirror)). `doc_summary_cap`
keeps its key and default and now reads as sentences. No knobs are open;
rejected alternatives live in `design.md`; changing any decision requires
a new spec delta.
