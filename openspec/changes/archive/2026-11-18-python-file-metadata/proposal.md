# Proposal: python-file-metadata

## WHY

The field test across the charly-vibes fleet (`sad-h3i`) reports 235
`comment_outside_channel` findings on 204 files that are Python shebang
lines (`#!/usr/bin/env python3`) or PEP 263 coding cookies
(`# -*- coding: utf-8 -*-`). `tree-sitter-python` yields both as ordinary
`comment` nodes, and `classify` routes them to prose because the Python
header channel is the module docstring only — no channel accepts mandatory
file metadata. Every executable Python script therefore fails the gate for
text the interpreter requires.

## What Changes

- **Python file metadata is an exemption.** A `#!` shebang on source line 1
  and a PEP 263 coding cookie (`# -*- coding: utf-8 -*-`, `# coding: utf-8`,
  `# coding=utf-8`) on source lines 1-2 attribute to
  `AttributionClass::Exemption`: exempt-but-observable, counted by
  `exemptions::summary`, never a `comment_outside_channel` finding.
- **Position is the gate, not the body.** `is_shebang` and
  `is_coding_cookie` are pure predicates in `attribution`; the positional
  gate (row 0 for a shebang, rows 0-1 for a cookie) lives in
  `extract::Extractor::push_comment` before `classify`, so `classify` stays a
  pure function of body and kind and its precedence is unchanged.
- **Exempt, not silent.** A file whose only comments are shebang + cookie has
  `exempt: true`, like an SPDX-only file. Shebang + cookie + narration keeps
  the narration finding and is not file-exempt. The same text after its
  allowed rows stays finding-eligible prose.

## Impact

- Spec: MODIFIES `Channel attribution` in `openspec/specs/comment-gate/spec.md`.
- Code: `sad/src/attribution.rs`, `sad/src/extract.rs`.
- Tests: `sad/tests/check_prose_test.rs`,
  fixture `sad/tests/fixtures/checks/prose/python_metadata.py`.
- Ticket: `sad-h3i`.