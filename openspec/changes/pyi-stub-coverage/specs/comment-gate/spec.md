# spec delta

## MODIFIED Requirements

### Requirement: Language frontends

The system SHALL support Rust, Python, TypeScript, and JavaScript via
pinned tree-sitter grammars (pretender's known-good pins) in v1. The
supported extension set SHALL be exactly `.rs`, `.py`, `.pyi`, `.ts`, `.js`,
and `.mjs`; `.pyi` stub files SHALL be parsed with the Python frontend, and
JavaScript files SHALL be parsed with the TypeScript frontend. Adjacent
extensions the gate cannot parse (`.tsx`, `.mts`, `.cts`, `.jsx`) SHALL be
discovered, skipped, counted in `files_unsupported`, and reported as envelope
warnings naming each path, never dropped silently. The set and its exclusions
SHALL be documented in `docs/src/gate-model.md`.

#### Scenario: Per-language fixture conformance

- **WHEN** the per-language fixture suite runs (one fixture exercising
  every attribution class per language)
- **THEN** each language produces the expected finding/envelope set

#### Scenario: JavaScript sources are gated

- **WHEN** a tree contains a `.js` and a `.mjs` file with prose comments
- **THEN** both are discovered, parsed with the TypeScript frontend, and
  their prose yields `comment_outside_channel` findings

#### Scenario: Unsupported adjacent extensions are visible

- **WHEN** a tree contains `.tsx`, `.mts`, `.cts`, and `.jsx` sources
- **THEN** each is counted in `files_unsupported` and named by one
  `unsupported_extension` envelope warning, and none produces findings

#### Scenario: Stub sources are discovered

- **WHEN** a tree contains a `.pyi` stub
- **THEN** discovery reports it as a supported source file and never counts
  it in `files_unsupported`

#### Scenario: Stub sources are analyzed as Python

- **WHEN** a `.pyi` stub is analyzed
- **THEN** it is parsed with the Python frontend and its docstrings and
  prose yield the same findings as the equivalent `.py` source