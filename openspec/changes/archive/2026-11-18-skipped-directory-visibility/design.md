# Design: skipped-directory-visibility

## Channel: an envelope warning, not a stat and not a failure

The envelope already reports per-path coverage loss through `warnings`
(`unsupported_extension`, `unreadable_source`). Reusing that channel with
`rule_name = "skipped_source_directory"` gives JSON consumers one mechanism
for every case where a source was not judged, names the directory through
`entity_id`, and needs no new stats key or exemption class. A counter would
say "some directories were skipped" without saying which, and failing the run
would turn a naming mistake in the tree into gate debt for every repository
that ships a nested `build/`.

## Ambiguous subset, depth rule

Only `target`, `build`, `dist`, `book`, and `out` are plausible
hand-written source-directory names. `node_modules`, `.venv`, `vendor`,
`third_party`, the caches, and `.git` are machine trees; a nested occurrence
carries no policy question, so warning about them would be noise.

Depth is the discriminator because a root-level `target/` or `build/` is the
conventional location of build output, while the same name below the root is
the case the ticket describes (`src/target/foo.rs`). Discovery therefore
records a directory only when its repo-relative path contains a separator.

## Paths only: no enumeration, no extra traversal

Discovery records the directory path it refused (forward-slashed, trailing
`/`, sorted). Listing the files inside would walk exactly the tree the walk
already decided not to traverse, and a vendor tree can be arbitrarily large.
The warning names the directory; if a source directory was misnamed, the
remediation tells the author to rename it, and the next run judges it.

## Rejected alternatives

- Warning on every skipped name: buries the ambiguous case in noise from
  `.git`, `.venv`, and the caches.
- Reporting root-level build trees too: every ordinary Rust or JS repository
  would carry a warning for its own `target/`.
- A `stats.directories_skipped` counter: one count cannot name the path, and
  the existing warnings already carry counts implicitly.