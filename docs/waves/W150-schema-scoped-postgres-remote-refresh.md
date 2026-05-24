# W150 Schema Scoped Postgres Remote Refresh

## Why

Remote refresh was keyed off the database-global WAL head. Unrelated writes in
the same Postgres database could therefore force one full VENOM rebuild even
when the VENOM schema itself had not changed.

## What changed

- Added one schema-local `change_watermark` table and trigger function.
- Installed statement-level triggers over VENOM durable tables to bump one
  shared schema-local sequence on write.
- Remote-change probes and rebuild observation now use that schema-local
  watermark instead of the global WAL head.

## Verification

- `cargo check -p venom-api --all-features`

