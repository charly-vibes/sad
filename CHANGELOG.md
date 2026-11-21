# Changelog

All notable changes to sad. The format is loose — this project versions by
milestone, not by cadence.

## Unreleased

- **Hook wiring** (sad-t7y, sad-pcn, sad-0j6): `sad hooks install` wires a
  marker-owned managed block (`sad-check: run: sad check`) into lefthook's
  pre-commit and pre-push stages — idempotent (`already_wired`), safe against
  a doubled gate (`skipped_user_gate` when a stage already runs the gate
  outside the block), and never creates a config. `sad hooks uninstall`
  removes the block marker-guarded from both stages (`removed` /
  `not_wired`), preserves foreign managed blocks byte-for-byte, and refuses
  unbalanced markers or a block outside the pinned stages instead of
  guessing. How-to: [Hook Wiring](docs/src/hooks.md).

## v0.2.0 — 2026-11-21

- **Published to crates.io as `sad-vibes`** (sad-tiy): the registry's `sad`
  name belongs to an unrelated 2021 placeholder crate, so the package takes
  the `sad-vibes` name while the lib/bin targets keep `sad` (genesis
  convention). Tagged releases now run `cargo publish --locked` alongside
  the binary archives.
- **Marker id consistency** (sad-o41): `unresolvable_marker` now extracts
  the cited id with the same attribution extractor that classified the
  node, so `see sad-lcy-` or `see sad-lcy(sic)` is judged as `sad-lcy`
  instead of the literal token — no false findings for trailing-hyphen or
  glued-punctuation cites.
- **Discovery**: a broken (unreadable or non-UTF-8) root `.gitignore`
  contributes no rules while deeper ignore files stay in force, exactly
  as the spec states; previously it disabled ignore filtering entirely.

- **Severity on every finding** (change `comment-smell-triage`): findings
  carry `severity: error|warning`, and `smell` names the prose smell —
  `narration`, `apology`, `todo`, `banner`, `commented_out_code`,
  `doc_cap`, `unclassified` — or is `null` for structural findings
  (`missing_header`, `unresolvable_marker`,
  `suppression_cap_exceeded`). Errors: narration, apology, todo,
  `commented_out_code`, and the structural findings. Warnings: banner,
  unclassified, and doc cap.
- Smell classifier: one deterministic post-pass over each maximal
  adjacent prose run, so a multi-line comment is judged as a unit while
  every member node keeps its own stable ID. Rules cover TODO/FIXME/XXX
  keywords, apology phrases (a bare "temporary" or "hack" is not
  enough), narration (a shared distinctive-token set with the next
  statement in scope, or a sequencer plus a named call), banners, and an
  `unclassified` fallback.
- Rationale is a channel (the fourth): a leading `SAFETY:`, `INVARIANT:`,
  `SECURITY:`, or `WHY:` tag with a non-empty body is counted in
  `stats.comments_rationale` and is not debt; `NOTE:` stays prose.
  Attribution precedence is fixed: header > doc > marker > rationale >
  suppression > exemption > prose.
- Doc cap measured in sentences: `doc_summary_cap` now counts sentences
  in the summary lead (text up to the first blank line) instead of
  rendered lines. One sentence passes however it wraps; inline code,
  URLs, abbreviations and version tokens do not split a sentence;
  sections after the first blank line are uncapped.
- `--strict` fails on any finding; `stats.findings_blocking` counts the
  new error-severity findings. The verdict now blocks on errors only, and
  adoption covers warnings as well as errors.
- Text mode names the split: `sad check — pass: 0 findings in M files`,
  `sad check — pass: 0 blocking, W warnings in M files`, or
  `sad check — fail: B blocking, W warnings in M files; <first>`.
- **BREAKING:** `ok` and the exit status now derive from error-severity
  findings, so warning-only runs pass (0.1.0 failed on any finding).
  Consumers that treated any finding as a failure must pass `--strict`
  or read `findings_blocking`. Finding IDs are unchanged and existing
  baselines carry over; a classifier change that promotes an adopted
  warning to error stays grandfathered until re-adoption.
  `doc_summary_cap` keeps its key and default (`1`), now read as
  sentences; see [docs/src/gate-model.md](docs/src/gate-model.md).
- **Python file metadata is exempt** (`sad-h3i`): a `#!` shebang on
  source line 1 and a PEP 263 coding cookie (`-*- coding: utf-8 -*-`,
  `coding: utf-8`, `coding=utf-8`) on source lines 1–2 attribute to the
  exemption class and are counted by the exemption summary instead of
  firing `comment_outside_channel`; the same text on a later line stays
  finding-eligible prose.
- **`@ts-expect-error` is suppression debt** (`sad-nw1`): the directive
  joins the recognised suppression forms as `ts_expect_error`, unscoped
  like `@ts-ignore`, so it is counted in `suppression_debt` and never a
  prose finding.
- **Licence lines beside SPDX are exempt** (`sad-6z5`): a comment body
  opening with `Copyright` plus a four-digit year, `(c)`, or `©`; a
  leading `©` or `(c)`; `Licensed under`; `License:`; or
  `All rights reserved` joins the licence header block. Prose that merely
  names a copyright or a licence stays a finding.
- **Skipped source directories are visible** (`sad-xkz`): the skip list
  grows to 21 names (adding `vendor`, `third_party`, `.mypy_cache`,
  `.ruff_cache`, `out`, `.turbo`, `.svelte-kit`), and a nested `target`,
  `build`, `dist`, `book`, or `out` directory at depth two or deeper emits
  one `skipped_source_directory` warning naming the path. A root-level
  build tree stays silent, and warnings never change the exit status.
- **Gitignored sources are out of scope** (`sad-z6q`): inside a git work
  tree, discovery honors root and nested `.gitignore` files with no
  configuration — no git binary, no index, no user or global excludes.
  Matching supports `!` negation, `/` anchoring, trailing-`/`
  directory-only rules, and `**`; an ignored directory is pruned, so a
  negation cannot re-include below it. Ignored paths add no counter and no
  warning; extension selection runs after ignore pruning. The file list
  stays a pure function of tree bytes plus path, so a branch switch that
  changes `.gitignore` content can change which sources are gated.
- **`.pyi` stubs are gated** (`sad-7hx`): the supported extension set is
  `.rs`, `.py`, `.pyi`, `.ts`, `.js`, and `.mjs`; stubs are parsed as
  Python with the existing doc channel and no stub-specific rule.
- **`commented_out_code` precision restored** (`sad-jdw`): a matched
  non-banner block is an `error` again. A body with no alphanumeric
  character never enters the class, the parse root must carry a
  declaration or control statement, and call, assignment, new-expression,
  formula, and annotation roots stay prose; a separator block reclassified
  as a banner stays a warning.

## v0.1.0 — unreleased

- Scaffold: Rust workspace, genesis-envelope CLI skeleton (`sad version`),
  family gates wiring (just/testaruda TDD loop, lefthook pre-commit gates,
  mdBook docs, CI).
- Gate model specified (2026-11-16 grill-me interview) —
  [docs/src/gate-model.md](docs/src/gate-model.md).
- Comment gate implemented (EARS change `specify-gate-objective`):
  tree-sitter frontends for Rust, Python and TypeScript; channel attribution
  (header / doc / marker / suppression / exemption / prose); checks
  `comment_outside_channel`, `doc_summary_cap_exceeded`,
  `unresolvable_marker`, `missing_header`, `suppression_cap_exceeded`;
  suppression debt and exemption counts in the genesis `check` envelope;
  `Assisted-by`-scoped default mode with `--all`; hardcoded per-kind
  remediation; `--adopt` baseline with shrink-only re-adoption;
  `sad.toml` configuration.
- Self-hosted: the repository gates itself (`sad.toml`,
  `sad-baseline.json`, `self-gate` recipe in `just ci`); 24 espectacular
  contracts map the EARS scenarios to tests; `just ci` runs spec
  correspondence, the pretender structural gate, the self-gate and spelling.
- Commented-out code is its own block-scoped finding
  (`commented_out_code`, on by default): maximal adjacent prose runs are
  normalised, parsed with the pinned frontend, and emitted once per block
  with `git log -p --follow <path>` remediation, replacing per-node prose
  only for matched blocks and failing closed otherwise.
- Change `specify-gate-objective` archived (2026-11-18): the EARS spec is
  deployed at `openspec/specs/comment-gate/spec.md` and its contracts at
  `.espectacular/comment-gate/`; `just spec-check` now runs the deployed
  `ah check`.
- Measurement duty recorded in [docs/src/validation.md](docs/src/validation.md)
  (first published rates + falsification rule).
- Release pipeline: tag-triggered `v*` workflow
  (`.github/workflows/release.yml`) builds five targets (Linux and macOS
  x86_64+aarch64, Windows x86_64), publishes archives, `checksums.txt` and
  `provenance.json` to the GitHub release, publishes the crate, and updates
  the Homebrew tap and Scoop bucket manifest; the README install section now
  covers prebuilt archives, Homebrew, Scoop and building from source.
