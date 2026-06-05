# W226. Binding Delta And System Event Window Tightening

Wave: `W226-binding-delta-and-system-event-window-tightening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce the remaining structural cost of inventory and operator-event refresh by
making component-binding refresh truly incremental and by avoiding redundant
`system events` window materialization on local merge paths.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W226-S01` | done | make component-binding inventory refresh use dedicated delta cursors instead of whole-subgraph reloads | `cargo test -p venom-api detached_postgres_read_snapshot_reloads_component_binding_sub_lanes_incrementally --all-features --offline` |
| `W226-S02` | done | expose merged and delta `system events` windows without rematerializing public vectors twice | `cargo test -p venom-domain system_event_query_index_merged_with_recent_windows_reuses_one_window_build --all-features --offline` |
| `W226-S03` | done | reuse the tighter `system events` domain helpers from the local merged snapshot cache | `cargo test -p venom-api local_merged_system_event_snapshot_reuses_domain_merged_windows --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
