# comment-gate specification delta

## MODIFIED Requirements

### Requirement: Channel attribution

The system SHALL parse source files with pinned tree-sitter grammars and
SHALL attribute every comment or doc-string node to exactly one class in
this precedence: file-header, doc, marker, rationale, suppression,
exemption, finding-eligible prose. A leading `SAFETY:`, `INVARIANT:`,
`SECURITY:`, or `WHY:` token with a non-empty body SHALL be rationale,
counted in `stats.comments_rationale`; `NOTE:` SHALL remain prose.
Attribution SHALL be purely structural (node kind, position, leading
token), never semantic. A Python `#!` shebang on source line 1 and a PEP
263 coding cookie — `# -*- coding: utf-8 -*-`, `# coding: utf-8`, or
`# coding=utf-8` — on source lines 1-2 SHALL be attribution-class
exemption: exempt-but-observable like an SPDX tag, counted by the
exemption summary, and never finding-eligible prose. The same text on a
later source line SHALL remain finding-eligible prose.

#### Scenario: Rust file with one node per class

- **WHEN** a Rust file contains a `//!` header block, a `///` doc on a
  `pub` item, a `see sad-lcy` marker comment, a
  `// SAFETY: the lock is held` rationale, an SPDX licence block, and an
  inline narration comment
- **THEN** attribution assigns each node to header / doc / marker /
  rationale / exemption / prose respectively

#### Scenario: Rationale tag requires a body

- **WHEN** a comment reads `// SAFETY:` with no text, or
  `// NOTE: the lock is held`
- **THEN** neither is rationale; both remain finding-eligible prose

#### Scenario: Precedence keeps doc and header above rationale

- **WHEN** a file contains `//! SAFETY: the buffer is pinned` and a
  `/// WHY: callers rely on this order`
- **THEN** the first is header and the second is doc, not rationale

#### Scenario: TypeScript and Python parity

- **WHEN** equivalent fixtures are analyzed for TypeScript (`/** */` on
  an exported symbol, module header block comment) and Python (docstring
  on an exported symbol, module docstring)
- **THEN** attribution produces the same class assignment pattern as the
  Rust fixture

#### Scenario: Python shebang and coding cookie are file metadata

- **WHEN** a Python file's first line is `#!/usr/bin/env python3` and its
  second line is a PEP 263 coding cookie, with no other comments
- **THEN** both nodes are attribution-class exemption, the exemption
  summary counts them, no `comment_outside_channel` finding is emitted,
  and the file reports `exempt: true`

#### Scenario: Shebang after the first line stays prose

- **WHEN** a `#!/usr/bin/env python3` comment appears on any source line
  after line 1
- **THEN** it remains finding-eligible prose and emits a
  `comment_outside_channel` finding

#### Scenario: File metadata does not silence other prose

- **WHEN** a Python file carries a shebang, a coding cookie, and a
  narration comment
- **THEN** the metadata lines stay exemption-class, the narration still
  emits its `comment_outside_channel` finding, and the file is not
  `exempt: true`