> *"It's sad (So sad), so sad*
> *It's a sad, sad situation*
> *And it's gettin' more and more absurd*
> *It's sad (So sad), so sad*
> *Why can't we talk it over?"*
> — Elton John

# Status

**v0.1 gate implemented** — the comment gate runs: tree-sitter frontends
for Rust, Python (including `.pyi` stubs), TypeScript and JavaScript;
six checks (`comment_outside_channel`, `commented_out_code`,
`doc_summary_cap_exceeded`, `unresolvable_marker`, `missing_header`,
`suppression_cap_exceeded`) plus `suppression_debt`; agent-attribution
scoping; genesis `check` envelope;
`--adopt` baseline with shrink-only re-adoption. The repository dogfoods it
(`sad.toml`, `sad-baseline.json`); `just ci` runs the gate through its
`self-gate` recipe (`cargo run -q --package sad-vibes -- check --all`), not as a
pre-commit hook.

The **gate design** is specified — see the [Gate Model](gate-model.md). The
EARS spec is deployed at
[`openspec/specs/comment-gate/spec.md`](https://github.com/charly-vibes/sad/blob/main/openspec/specs/comment-gate/spec.md)
(change archived 2026-11-18, with 24 property-derived contracts under
`.espectacular/comment-gate/`). Ticket `sad-lcy` is closed. Live command surface:
`sad version` and `sad check [--all] [--adopt] [--strict] [--config <path>] [--json]`.
Measurement duty and first published rates: [Validation](validation.md).
The kaya-compiled record ships with the book: the client-facing
[publication overview](public/overview.md) and the
[research dossier](research/index.md) (rendered at build time by
`scripts/build-publications.py`).
Implementation history lives in
[CHANGELOG](https://github.com/charly-vibes/sad/blob/main/CHANGELOG.md).
