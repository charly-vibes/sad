> *"It's sad (So sad), so sad*
> *It's a sad, sad situation*
> *And it's gettin' more and more absurd*
> *It's sad (So sad), so sad*
> *Why can't we talk it over?"*
> — Elton John

# Getting Started

`sad` is a deterministic gate on in-code prose — comments and doc strings.
Instead of asking for more docs, it flags where prose is doing work the code
should do: prose is treated as evidence the code failed to express itself —
the fix is a refactor into simpler, more expressive code, not more words.

> The gate design is [specified](gate-model.md); implementation is under
> tickets and not shipped.

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
sad check [--json] [--all] [--adopt] [--config <path>]   # gate run (skeleton)
```

Every command emits the suite JSON envelope — key fields `ok`, `data`,
`warnings`, `hints` (full schema: the genesis envelope contract). `--json`
prints that envelope raw; text mode is a convenience rendering of the same
envelope. Exit status mirrors `ok` — a failed gate is never process success.

## Gate checks

The gate surface is specified in [Gate Model](gate-model.md).

### Adopting the gate (once shipped)

On an existing tree, adopt with the full scope:

```bash
sad check --all --adopt
```

Adoption must run with `--all`: the default attribution scope analyzes only
files whose governing revision carries the configured attribution trailer,
so legacy prose would stay unbaselined and the first agent edit of a legacy
file would surface all of it as new findings. Commit the resulting
`sad-baseline.json`. `--adopt` writes the baseline and then evaluates
against it, so the adoption run exits 0; later runs exit 1 on findings the
baseline does not cover, or on a gate or configuration failure.

Config discovery: `--config <path>`, else `./sad.toml`, else built-in
defaults (see the [configuration table](gate-model.md#configuration-sadtoml)).

### Pre-commit hook

Merge the command into your existing `pre-commit.commands` — do not add a
second top-level `pre-commit` key:

```yaml
sad-gate:
  run: sad check
```

The default attribution scope applies here (see
[Scope](gate-model.md#scope)).

### CI

Steps for a consuming repository. sad has no crates.io release, so the
install comes from the git repository:

```yaml
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install sad
        run: cargo install --git https://github.com/charly-vibes/sad --locked --package sad
      - name: sad gate
        run: sad check --all
```

`--all` is required in CI as well: committed legacy prose is otherwise out
of scope.

## Development

```bash
just test      # testaruda-driven TDD loop (affected tests only)
just test-full # plain cargo test — no family tooling needed
just ci        # fmt + type-check + lint + test — same gate as CI
```
