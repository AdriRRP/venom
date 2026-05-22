# W132. Incremental Read-Side Refresh

Wave: `W132-incremental-read-side-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Refresh inventory-backed and findings-backed snapshot lanes separately so one
mutation path does not deep-clone unrelated read-side state.

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W132-S01` | done | split snapshot-cache refresh helpers by inventory vs read-model concern and use them on hot governance/ingestion paths | `unit`, `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`
