# Design: commented-out-code-precision

## Predicate

Precision is a root-shape rule, not another phrase blacklist. After the
clean-parse, non-empty-root, and root-allow checks:

1. A body with no alphanumeric character is rejected by the caller before
   the parser. A separator cannot be code, so it need not pay a parse.
2. At least one root child must be in the language's closed `code_shape`
   table. The table holds declarations and control statements only.
   Rust wraps control expressions in `expression_statement`, so that
   payload is unwrapped for the lookup; Python and TypeScript name control
   statements directly at the root.
3. Root-deny and payload-deny still outrank a code-shaped sibling, and an
   unknown kind stays absent from every table, so the predicate fails
   closed to prose.

Bare expression roots — `f();`, `x = 1;`, `new Foo();`, `E = m * c^2`,
`e.g. g();` — have no declaration or control child and stay prose. This
matches the field corpus: those shapes were documentation and test
annotations, not dead code. The rule is deliberately conservative about
false negatives too: a commented `return x;` or a bare macro invocation is
no longer claimed as dead code. The per-node prose finding still reports
it, so the gate never goes silent.

The chunking bounds and their fail-closed boundary stay exactly as they
were; only the per-chunk predicate is narrower.

## Severity

`commented_out_code` was warning-only because the predicate was imprecise.
Once the corpus above stays prose, a matched non-banner block is evidence
of dead code and is an error. The banner reclassification for
separator-bodied block findings remains, at warning severity, so a
separator that somehow reaches the check is still not blocking. Severity is
not part of finding identity: IDs are content hashes of the normalized
body, so adopting a baseline before this change keeps old IDs
grandfathered.

## Spec delta

OpenSpec's MODIFIED requirement replaces the whole block and refuses to drop
a scenario title the deployed spec still has. The two former titles that no
longer describe current behavior — `Block findings are warnings until
precision lands` and `Documentation call example is a known false positive` —
are kept as historical identifiers and re-worded to the surviving contract,
alongside the new `Block findings are errors` and `Call-shaped examples stay
prose` scenarios.

## Alternatives considered

- **Phrase blacklists for formulas and annotations.** Rejected: prose is
  unbounded and a blacklist would drift. The grammar already knows a root
  of bare expressions.
- **Reject any body without a top-level item (Rust only).** Rejected:
  `let x = 1;` is a true positive and has no item; declarations and
  control statements are the intended boundary.
- **Drop `root_allow` / payload-deny.** Rejected: they are the existing
  fail-closed guards for prose that happens to parse; narrowing only the
  shape rule keeps their near-miss behavior.