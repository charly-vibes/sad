# sad — documentation gate CLI
# https://just.systems

default:
    @just --list

# Install sad binary to ~/.cargo/bin
install:
    cargo install --path sad --locked

# Build (debug)
build:
    cargo build

# Build (release)
release:
    cargo build --release


# TDD loop (most obvious entry, DDL test-runner standard — see DDL-85p in
# the dulce-de-leche repo): testaruda exec — select the tests the working
# diff affects, run only those, ingest, calibrate. Tolerance contract:
# exit 0/10/20 green; NOT CALIBRATED with 0 actual failures is a vacuous
# pass, with failures it blocks. Local store (.testaruda/, gitignored)
# initializes on first use; a stale store self-heals (wild-z56).
test:
    #!/usr/bin/env bash
    set -uo pipefail
    if [ ! -d .testaruda ]; then
      testaruda init || { echo "❌ testaruda init failed" >&2; exit 1; }
    fi
    code=0
    testaruda exec || code=$?
    case "$code" in
      0|10|20) exit 0 ;;
      4)
        echo "⚠️ runner exit 4 — pruning stale local calibration store and retrying once (wild-z56)" >&2
        rm -rf .testaruda
        testaruda init || { echo "❌ testaruda re-init failed" >&2; exit 1; }
        testaruda exec ;;
      *) exit "$code" ;;
    esac

# Blast-radius preview: which tests a change WOULD affect — runs nothing.
# Useful before a refactor; optionally name files: just test-blast sad/src/foo.rs
test-blast *files:
    #!/usr/bin/env bash
    if [ -z "{{files}}" ]; then
      testaruda select --pre-edit --human
    else
      testaruda select --pre-edit --human --files "{{files}}"
    fi

test-full:
    cargo test

# Type-check without building (named to avoid confusion with a future
# `sad check` gate command)
type-check:
    cargo check

# Lint (deny warnings, all targets)
lint:
    cargo clippy --all-targets -- -D warnings

# Format source files
fmt:
    cargo fmt

# Format check (CI)
fmt-check:
    cargo fmt -- --check

# Spell check (typos, configured in _typos.toml)
typos:
    typos

# Generate Rust API docs
doc:
    cargo doc --no-deps --open

# Build user-facing docs (requires: cargo install mdbook)
book:
    mdbook build

# Serve docs locally with live reload (requires: cargo install mdbook)
book-serve:
    mdbook serve --open

# Clean build output
clean:
    cargo clean

# Spec-test correspondence (espectacular): every scenario has a contract and
# every contract maps to a scenario. Uses the in-flight change overlay while
# the change is open; after archive the deployed specs are checked.
spec-check:
    #!/usr/bin/env bash
    set -uo pipefail
    if [ -d openspec/changes/specify-gate-objective ]; then
      ah check --changes specify-gate-objective
    else
      ah check
    fi

# Structural gate (pretender): complexity/duplication on this repo.
structure-check:
    pretender check --mode gate

# Dogfood: sad gates its own tree against sad-baseline.json.
self-gate:
    cargo run -q --package sad -- check --all

# Full CI gate: fmt + type-check + lint + tests + spec correspondence +
# structural gate + self-gate + spelling
ci: fmt-check type-check lint test spec-check structure-check self-gate typos

# Project-local preflight gate.
# Prefer this over `bd preflight` for now; the current beads preflight is
# Go-specific upstream and does not reflect this Rust workspace.
preflight: ci

# Show project status
status:
    wai status
    bd ready
