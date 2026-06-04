# W223. Postgres Runtime And System Event Cost Tightening

Wave: `W223-postgres-runtime-and-system-event-cost-tightening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Remove one more fixed Postgres resident lane from steady state and tighten the
remaining hot-path allocation and merge costs around operator-facing `system
events`, so the reliability substrate stays truthful while moving closer to the
product's lean steady-state target.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W223-S01` | done | make the Postgres runtime lane ephemeral until taken so `ApiState` keeps one live resident service by default | `cargo test -p venom-api postgres_runtime_lane_is_ephemeral_until_taken --all-features --offline` |
| `W223-S02` | done | remove avoidable public-window cloning from `SystemEventQueryIndex` delta and merge paths | `cargo test -p venom-domain system_event_query_index_delta_since_uses_borrowed_recent_windows --all-features --offline` |
| `W223-S03` | done | tighten local merged `system events` fallback paths so cached peer windows and merged indices avoid redundant full window materialization | `cargo test -p venom-api local_merged_system_event_snapshot_reuses_cached_peer_window --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
