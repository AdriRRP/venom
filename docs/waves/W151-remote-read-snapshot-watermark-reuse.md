# W151 Remote Read Snapshot Watermark Reuse

## Why

Once the HTTP layer knew the mutable Postgres-backed store was stale, repeated
fresh reads could still keep rebuilding the same detached remote snapshot until
the write store caught up.

## What changed

- Cached the last detached remote read snapshot by schema-local watermark.
- Fresh reads now reuse that snapshot until the remote watermark changes again.
- The mutable write store remains independently stale until it performs its own
  pre-mutation refresh.

## Verification

- `cargo check -p venom-api --all-features`

