# Design: licence-header-lines

## Decision

Licence text beside an SPDX tag belongs to the licence exemption. A comment
body that opens with one of the following bounded shapes is exemption-class
and contributes to the file's `exempt` flag exactly like an SPDX tag:

| shape | example |
|---|---|
| `Copyright` + four-digit year | `Copyright 2026 Charly Vibes` |
| `Copyright` + `(c)` | `Copyright (c) 2026 Charly Vibes` |
| `Copyright` + `©` | `Copyright © 2026 Charly Vibes` |
| leading `©` | `© 2026 Charly Vibes` |
| leading `(c)` | `(c) 2026 Charly Vibes` |
| `Licensed under` | `Licensed under the Apache License, Version 2.0` |
| `License:` | `License: MIT` |
| `All rights reserved` | `All rights reserved.` |

Matching is ASCII case-insensitive, the markers are anchored at the start of
the body, and the word-shaped markers end on a word boundary, so
`Licensed undergraduate work` and `All rights reservedly claimed` are prose.

## Why not the alternatives

- **Exempt every line that contains `Copyright`/`license` anywhere.** This
  swallows narration such as `// Copyright is a legal concept` and
  `// License negotiation happens elsewhere`, which is precisely the class
  the gate exists to catch. Rejected.
- **Exempt only inside a contiguous SPDX block.** It needs cross-node state
  in `is_exemption`, which is currently a pure per-body predicate, and it
  exempts a licence line only when adjacent to the tag. The bounded marker
  shape is enough and keeps attribution structural. Rejected.
- **Parse full licence boilerplate.** Unbounded and untestable against the
  corpus; the fleet shapes are the table above. Rejected.

## Accepted false-negative boundary

Prose that literally opens with one of the marker shapes — for example
`// Copyright is a legal concept` if it read `// Copyright 2026 is when the
term was defined` — is treated as licence text. The gate stays deterministic
and structural: the marker, not the sentence, decides. The alternative bias
(a false positive on real headers, as `sad-6z5` reports) is worse because it
makes mandatory licence text unadoptable.

`Copyright is a legal concept` does **not** match: `Copyright` must be
followed by a year, `(c)`, or `©`.

## Consequence for whole-file exemption

Because the predicate feeds `compute_exempt` unchanged, a file whose only
comments are an SPDX tag plus licence lines reports `exempt: true`, and a
file that still carries prose keeps node-level exemptions only. That matches
the deployed `Exemptions are observable` semantics; no new counter is added.