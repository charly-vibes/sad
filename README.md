> *"It's sad (So sad), so sad*
> *It's a sad, sad situation*
> *And it's gettin' more and more absurd*
> *It's sad (So sad), so sad*
> *Why can't we talk it over?"*
> — Elton John

# sad

> **Why:** documentation is too often an apology for code that failed to
> express itself — every comment and doc string is evidence the code is doing
> too much. sad gates on exactly that: deterministically flagging in-code
> prose doing work the code should do, so the fix is a refactor into
> simpler, more expressive code, not more words. Sibling of
> [pretender](https://github.com/charly-vibes/pretender) (structural
> complexity gates) and [vampiro](https://github.com/charly-vibes/vampiro)
> (composition proofs) — same CI-and-pre-commit gate family.
> **Status:** scaffold · [gate model specified](docs/src/gate-model.md) · [status](docs/src/status.md)
> · [charly-vibes Tool Ecosystem](https://charly-vibes.github.io/dulce-de-leche/ecosystem-map.html)
[![tracked with wai](https://img.shields.io/badge/tracked%20with-wai-blue)](https://github.com/charly-vibes/wai)
[![CI](https://github.com/charly-vibes/sad/actions/workflows/ci.yml/badge.svg)](https://github.com/charly-vibes/sad/actions/workflows/ci.yml)

sad is a deterministic gate on in-code prose — comments and doc strings.
No network, no models, no randomness: same input tree, same findings.

The premise: **code should be self-explanatory.** Writing docs and comments
to explain code means the code is doing too much and needs to be refactored
into simpler and more expressive code. sad turns that premise into a gate
that runs in CI and pre-commit hooks, so unexpressive code never merges.

## Install

From source (no release yet):

```bash
cargo install --path sad --locked
# or, from a clone:
just install
```

The [gate model](docs/src/gate-model.md) — three legitimate prose channels,
four checks, pretender-mirror output contract — is specified but not
implemented yet.

## Current command surface

```bash
sad version [--json]   # tool identity as a genesis suite envelope
```

Every command emits the suite JSON envelope; exit status mirrors `ok`.

## Development

Prerequisites: Rust toolchain and `just`. The fast loops also use family
tooling (`testaruda`, `typos`, `mdbook`, `lefthook`, plus the ecosystem
gates `ah` and `pretender`) — without `testaruda`, use `just test-full`
for a plain `cargo test`. Full setup: the
[getting started guide](docs/src/getting-started.md).

```bash
just test      # testaruda-driven TDD loop (affected tests only)
just ci        # fmt + type-check + lint + test — same gate as CI
just book      # mdBook user docs
```

## License

Apache-2.0 — see [LICENSE](LICENSE).
