# Tasks: commented-out-code-precision

TDD: the precision corpus and severity contract exist before the predicate
changes. The spec delta is this file's sibling
`specs/comment-gate/spec.md`; the field evidence is
`changes/archive/2026-11-18-comment-smell-triage/evidence.md`.

## sad-jdw — precision corpus

- [x] 1. Red: formula, expected-value annotation, bare call,
      documentation example, labeled statement, and separator bodies stay
      prose
- [x] 2. Red: declarations and control statements (`fn f() {}`,
      `let x = 1;`, `use a::b;`, `struct S;`, Python `for`/`import`/`def`,
      TypeScript `const`/`function`/`if`) keep firing
- [x] 3. Red: a bodyless signature and annotation-only assignment still
      outrank a code-shaped sibling

## sad-jdw — predicate

- [x] 4. Reject an alphanumeric-free body before the parser
- [x] 5. Add the closed per-language `code_shape` table and require at
      least one code-shaped root child; unwrap Rust `expression_statement`
      control payloads
- [x] 6. Remove the now-redundant tree-wide shape and argument-shape
      tables; keep root-allow, root-deny, payload-deny, and chunking
      unchanged

## sad-jdw — severity

- [x] 7. Red: a matched block carries `severity: error` before and after
      classification, with an unchanged ID
- [x] 8. Promote the block finding and `severity_of(Smell::CommentedOutCode)`
      to error; keep the separator banner at warning

## Contracts and gates

- [x] 9. Declare espectacular contracts for every new or modified scenario
      under `.espectacular/changes/commented-out-code-precision/comment-gate/`
- [x] 10. Run `cargo test --package sad --test check_commented_out_code_test`,
      `cargo test --package sad --test check_commented_out_code_predicate_test`,
      `cargo test --package sad --no-run`, and
      `openspec validate commented-out-code-precision --strict`
- [ ] 11. Archive the change and close `sad-jdw` (orchestrator)