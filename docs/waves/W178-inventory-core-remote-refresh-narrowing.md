# W178. Inventory-Core Remote Refresh Narrowing

Wave: `W178-inventory-core-remote-refresh-narrowing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Refresh only the changed inventory-core durable tables during detached Postgres
reads instead of reloading the whole inventory-core subgraph.

## Feature paths

- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W178-S01` | done | split detached inventory-core refresh so components, context profiles, and tag definitions reload only their own durable tables | `cargo test -p venom-api postgres_backend_reloads_component_context_profiles --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
