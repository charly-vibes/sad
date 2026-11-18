# Tasks: python-file-metadata

TDD: the red contract tests land with the spec delta in this change's
sibling `specs/comment-gate/spec.md`, then the implementation turns them
green.

## sad-h3i — Python file metadata

- [x] 1. Red: a Python file whose first line is a shebang and whose second
      line is a PEP 263 coding cookie has no `comment_outside_channel`
      findings, both nodes are exemption-class, and the file is
      `exempt: true`
- [x] 2. Red: a shebang on any later line is still a prose finding
- [x] 3. Red: shebang + cookie + narration keeps one prose finding and is
      not file-exempt
- [x] 4. Implement `is_shebang` and `is_coding_cookie` in
      `attribution.rs`; keep `classify` pure and its precedence unchanged
- [x] 5. Gate position in `Extractor::push_comment`: row 0 for a shebang,
      rows 0-1 for a cookie, Python only
- [x] 6. Confirm `exemptions::summary` counts the metadata nodes and that
      the file's `exempt` flag follows the existing licence-only rule

## Contracts and gates

- [x] 7. Declare espectacular contracts for the three new scenarios under
      `.espectacular/changes/python-file-metadata/comment-gate/`
- [x] 8. Run the targeted cargo tests and
      `openspec validate python-file-metadata --strict`