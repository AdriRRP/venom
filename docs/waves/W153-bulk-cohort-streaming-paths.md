# W153 Bulk Cohort Streaming Paths

## Why

The read model still exposed old vector-materializing bulk helpers even though
the durable write paths had already moved to visitor-style streaming.

## What changed

- Removed leftover vector-collecting bulk helpers from `FindingReadModel`.
- Kept visitor-based bulk traversal as the canonical path for governance
  cohorts.
- Updated the read-model test to assert full cohort coverage through the
  streaming visitor path.

## Verification

- `cargo test -p venom-domain finding_read_model --all-features`

