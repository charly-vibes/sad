# Proposal: licence-header-lines

## WHY

`is_exemption` recognises SPDX file tags and `@generated` only, while the
deployed `Exemptions are observable` requirement promises "licence/SPDX
header blocks". A file that carries

```text
// SPDX-License-Identifier: MIT
// Copyright 2026 X
```

exempts the SPDX line and reports the copyright line as
`comment_outside_channel` (`sad-6z5`). Forty-two fleet source files carry
SPDX tags, and any that pair a copyright line with the tag produce noise.

## What Changes

- **Licence text beside SPDX is licence text.** `is_exemption` gains a
  bounded `starts_with_licence_tag` predicate covering `Copyright` followed
  by a four-digit year, `(c)`, or `©`; a leading `©` or `(c)`;
  `Licensed under`; `License:`; and `All rights reserved`.
- **The predicate is deliberately narrow.** `Copyright is a legal concept`
  and `License negotiation happens elsewhere` stay finding-eligible prose,
  because the marker shape — a year, the copyright sign, or a fixed phrase —
  is required.
- **The boundary is documented.** Prose that literally opens with one of the
  marker shapes is exempt; that accepted false-negative boundary and the
  rejected alternatives are recorded in `design.md`.

## Impact

- Spec: MODIFIES `Exemptions are observable`.
- Code: `sad/src/attribution.rs` (`is_exemption`, `starts_with_licence_tag`).
- Tests: `sad/tests/check_prose_test.rs`.
- Ticket: `sad-6z5`.