> *"Hey, hey, hey*
> *I'm your life*
> *I'm the one who takes you there*
> *I'm the one who cares*
> *...*
> *Sad but true"*
> — Metallica

# sad

> **Why:** documentation is too often an apology for code that failed to
> express itself — every comment and doc string is evidence the code is doing
> too much. sad gates on exactly that: deterministically flagging where
> prose is doing work the code should do, so the fix is a refactor into
> simpler, more expressive code, not more words. Sibling of
> [pretender](https://github.com/charly-vibes/pretender) (structural
> complexity gates) and [vampiro](https://github.com/charly-vibes/vampiro)
> (composition proofs) — same CI-and-pre-commit gate family.
> **Status:** scaffold — objective being specified
> · [charly-vibes Tool Ecosystem](https://charly-vibes.github.io/dulce-de-leche/ecosystem-map.html)
[![tracked with wai](https://img.shields.io/badge/tracked%20with-wai-blue)](https://github.com/charly-vibes/wai)

sad is a deterministic documentation gate CLI — no network, no models, no
randomness: same input tree, same findings.

The premise: **code should be self-explanatory.** Writing docs and comments
to explain code means the code is doing too much and needs to be refactored
into simpler and more expressive code. sad turns that premise into a gate
that runs in CI and pre-commit hooks, so unexpressive code never merges.

## Install

From source (no release yet):

```bash
cargo install --path sad
```

## Current command surface

```bash
sad version [--json]   # tool identity as a genesis suite envelope
```

Every command emits the suite JSON envelope; exit status mirrors `ok`.

## Development

```bash
just test      # testaruda-driven TDD loop (affected tests only)
just ci        # fmt + type-check + lint + test — same gate as CI
just book      # mdBook user docs
```

## License

Apache-2.0 — see [LICENSE](LICENSE).
