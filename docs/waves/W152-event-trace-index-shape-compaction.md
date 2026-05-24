# W152 Event Trace Index Shape Compaction

## Why

Merging local `system event` windows still rebuilt one combined vector through
`chain + sort + truncate`, which was correct but needlessly allocation-heavy.

## What changed

- Replaced the old merge path with one bounded linear merge over already-sorted
  recent event windows.
- Kept the same observable ordering and bounded limit semantics.

## Verification

- `cargo test -p venom-domain finding_read_model --all-features`
- `cargo check -p venom-api --all-features`

