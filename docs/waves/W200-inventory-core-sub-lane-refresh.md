# W200. Inventory Core Sub Lane Refresh

Wave: `W200-inventory-core-sub-lane-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Split detached Postgres inventory-core refresh into narrower component,
context-profile, and component-tag sub-lanes so a small inventory-core change
does not reload unrelated durable tables.

## Feature paths

- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W200-S01` | done | refresh only the changed inventory-core detached sub-lanes instead of reloading the whole inventory-core graph | `cargo test -p venom-api detached_postgres_read_snapshot_reloads_inventory_core_sub_lanes_incrementally --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
