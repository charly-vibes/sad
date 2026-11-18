# Tasks: licence-header-lines

TDD: the red contract tests land with the spec delta in this change's
sibling `specs/comment-gate/spec.md` and the decision in `design.md`, then
the implementation turns them green.

## sad-6z5 — licence lines

- [x] 1. Red: an SPDX line paired with `Copyright 2026 …` yields two
      exemption nodes, `exempt: true`, and no `comment_outside_channel`
      finding
- [x] 2. Red: `Copyright is a legal concept` and `License negotiation …`
      remain prose findings (the bounded shape is required)
- [x] 3. Extend `is_exemption` with the bounded
      `starts_with_licence_tag` for the six documented shapes, keeping the
      predicate per-body and pure
- [x] 4. Record the decision and the accepted false-negative boundary in
      `design.md`
- [x] 5. Add attribution unit coverage for the bounded shapes and their
      near misses

## Contracts and gates

- [x] 6. Declare espectacular contracts for the two new scenarios under
      `.espectacular/changes/licence-header-lines/comment-gate/`
- [x] 7. Run the targeted cargo tests and
      `openspec validate licence-header-lines --strict`