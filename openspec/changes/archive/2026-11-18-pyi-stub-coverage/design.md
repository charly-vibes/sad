# Design: pyi-stub-coverage

## Decision

`.pyi` stub files are analysed as Python. `pyi` is added to
`SOURCE_EXTENSIONS` and `language_for_path` maps it to the Python frontend, so
the grammar that parses a module also parses its stub.

## Why the Python frontend, unchanged

Stub syntax is Python syntax: `def` with a `...` body, classes with annotated
attributes, `@overload` stacks, `if sys.version_info` guards, and imports are
all ordinary nodes to tree-sitter-python. No second grammar, no preprocessing,
and no stub marker in the AST is needed to walk them.

## Doc channel is unchanged

A stub's docstrings are module, class, and function docstrings — the same doc
nodes the channel model already counts, in the same positions a `.py` file
would have them. Prose outside those docstrings remains a finding, markers
and suppressions behave as in `.py`, and the header rule is unchanged. There
is no stub-specific rule because there is no stub-specific prose surface: the
public API documentation a stub carries is exactly the docstring channel.

## Rejected alternatives

- **Docstrings only.** A stub-only pipeline would need its own walk and its
  own eligibility rules, and it would still have to decide what a prose
  comment in a stub means. Reusing the Python frontend answers that question
  with the existing channel model.
- **Exclude stubs with a documented reason.** That leaves the most
  prose-heavy public-API surface ungated, which is the gap the ticket exists
  to close.
- **Treat stubs as unsupported.** Ignoring an extension the grammar already
  parses reports a language boundary that does not exist.