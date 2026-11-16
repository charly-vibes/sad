# Getting Started

`sad` is a deterministic gate on in-code prose — comments and doc strings.
Instead of asking for more docs, it flags where prose is doing work the code
should do: prose is treated as evidence the code failed to express itself —
the fix is a refactor into simpler, more expressive code, not more words.

> The gate design is [specified](gate-model.md) — implementation hasn't
> started. The command surface below is what exists today.

## Prerequisites

- **Required:** Rust toolchain (1.98+), `just`
- **Optional** (family tooling for the fast loops): `testaruda`
  (affected-test selection), `mdbook` (docs), `typos` (spell check),
  `lefthook` (pre-commit gates), plus the ecosystem gates `ah` and
  `pretender`

Without `testaruda`, use `just test-full` — a plain `cargo test`.

## Build from source

```bash
git clone https://github.com/charly-vibes/sad
cd sad
just install    # cargo install --path sad --locked
sad --version
```

## Current command surface

```bash
sad version [--json]   # tool identity as a genesis suite envelope
```

Every command emits the suite JSON envelope — key fields `ok`, `data`,
`warnings`, `hints` (full schema: the genesis envelope contract); text mode
is a convenience rendering of the same envelope. Exit status mirrors `ok` —
a failed gate is never process success.

## Development

```bash
just test      # testaruda-driven TDD loop (affected tests only)
just test-full # plain cargo test — no family tooling needed
just ci        # fmt + type-check + lint + test — same gate as CI
```
