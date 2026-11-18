# Changelog

All notable changes to sad. The format is loose — this project versions by
milestone, not by cadence.

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
