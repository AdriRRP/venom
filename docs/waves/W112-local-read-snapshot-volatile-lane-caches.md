# W112. Local Read Snapshot Volatile Lane Caches

Wave: `W112-local-read-snapshot-volatile-lane-caches`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Cache local `system events` and `command statuses` as `Arc` lanes inside the
durable local stores.

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/app/service.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W112-S01` | done | replace rebuild-on-read of volatile local lanes with cached `Arc` snapshots | `unit`, `integration` |

## Invariant impact

`I8`, `I11`
