# Getting Started

`sad` is a deterministic documentation gate: instead of asking for more docs,
it flags where prose is doing work the code should do. Comments and doc
strings are treated as evidence the code failed to express itself — the fix
is a refactor into simpler, more expressive code, not more words.

> This page is a scaffold — the gate design is being specified. The command
> surface below is what exists today.

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

Every command emits the suite JSON envelope (`ok`, `data`, `warnings`,
`hints`); text mode is a convenience rendering of the same envelope. Exit
status mirrors `ok` — a failed gate is never process success.

## Development

```bash
just test      # testaruda-driven TDD loop (affected tests only)
just ci        # fmt + type-check + lint + test — same gate as CI
```
