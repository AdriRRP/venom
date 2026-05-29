# W210. Inventory Core Delta Refresh

Wave: `W210-inventory-core-delta-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Stop reloading full `inventory-core` and collection-definition subgraphs during
Postgres refreshes when only a bounded set of identities changed.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W210-S01` | done | refresh inventory-core and collection-definition snapshots from changed identities instead of full subgraph reloads | `cargo test -p venom-api detached_postgres_read_snapshot_reloads_inventory_core_from_changed_identities --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
