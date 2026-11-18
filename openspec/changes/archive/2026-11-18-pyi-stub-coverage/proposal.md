# Proposal: pyi-stub-coverage

## WHY

The fleet holds 12 `.pyi` files and the gate cannot see any of them:
`SOURCE_EXTENSIONS` is `rs|py|ts|js|mjs`, so stubs land in the
`unsupported_extension` bucket, or — before that bucket existed — nowhere.

That gap matters more than a missing extension usually would. A `.pyi` stub is
where a Python package states its public API, and the channel model says
public-API doc-comments are the one legitimate prose channel. Leaving stubs
ungated means the documented-prose surface most likely to hold prose is the
surface the gate cannot read.

## What Changes

- `.pyi` stubs SHALL be analysed as Python: `pyi` joins
  `SOURCE_EXTENSIONS` and `language_for_path` maps it to the Python frontend,
  which parses stubs with tree-sitter-python.
- The doc channel SHALL be unchanged: module, class, and function docstrings
  are doc comments where present, and no stub-specific rule is introduced.
  `...` bodies, `@overload` stacks, and conditional definitions are ordinary
  Python to the grammar.
- Discovery SHALL count a stub as a supported source file, not an
  unsupported extension, and the `unsupported_extension` warning text SHALL
  list the widened set.

## Impact

- Spec: `openspec/specs/comment-gate/spec.md` modifies the
  *Language frontends* requirement (extension set, frontend mapping) and adds
  two scenarios.
- Code: `sad/src/check/discovery.rs` (`SOURCE_EXTENSIONS`),
  `sad/src/extract.rs` (`language_for_path`), `sad/src/check.rs`
  (`unsupported_extension` message text).
- Tests: `pyi_stubs_are_discovered` (discovery) and
  `pyi_stubs_are_analyzed_as_python` (extraction).
- Ticket: `sad-7hx`.

## Out of scope

- A stub-specific comment rule. A stub's prose is judged by the same channel
  model as its `.py` implementation.
- Type checking, stub completeness, or comparing a stub against its
  implementation.
- `.pyw`/`.ipynb` and other Python-adjacent extensions; the ticket inventory
  names stubs only.