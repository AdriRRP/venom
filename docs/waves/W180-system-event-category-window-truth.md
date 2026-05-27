## Why

`system events` category queries became semantically false after reducing the
index to one global recent window. A sparse category could report truthful
historical totals while returning an empty first page.

## Scope

- keep one retained event store
- restore truthful recent category pages
- avoid duplicating full event payloads per category window

## Slices

### W180-S01 single-retention category windows

Status: done

- extend the event query index with per-category recent links over the shared
  retained event store
- rebuild truthful category pages from those links
- reload Postgres recent windows with both global and category ranks

## Verification

- `cargo test -p venom-domain system_event_trace --all-features --offline`

