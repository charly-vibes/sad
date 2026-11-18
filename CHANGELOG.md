# Changelog

All notable changes to sad. The format is loose — this project versions by
milestone, not by cadence.

## v0.2.0 — unreleased

- **Severity on every finding** (change `comment-smell-triage`): findings
  carry `severity: error|warning`, and `smell` names the prose smell —
  `narration`, `apology`, `todo`, `banner`, `commented_out_code`,
  `doc_cap`, `unclassified` — or is `null` for structural findings
  (`missing_header`, `unresolvable_marker`,
  `suppression_cap_exceeded`). Errors: narration, apology, todo, and the
  structural findings. Warnings: banner, unclassified, doc cap, and
  `commented_out_code` until `sad-jdw` restores its precision.
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
