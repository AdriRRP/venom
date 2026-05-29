# W215. Collection Lane Delta Refresh

Wave: `W215-collection-lane-delta-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Refresh Postgres collection lanes from durable deltas for sources, memberships,
and schedules so collection mutations stop reloading whole collection
subgraphs.

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
| `W215-S01` | done | replay collection source, membership, and schedule deltas from durable cursors | `cargo test -p venom-api detached_postgres_collection_refresh_reloads_only_changed_collections --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
