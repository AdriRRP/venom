# W206. Collection Sub-Lane Refresh

Wave: `W206-collection-sub-lane-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Split collection refresh into narrower durable sub-lanes so detached and live
reloads only touch the changed collection subgraph instead of reloading every
collection-owned table together.

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
| `W206-S01` | done | reload collection definitions, sources, memberships, and schedules through narrower change lanes | `cargo test -p venom-api detached_postgres_read_snapshot_reloads_collection_sub_lanes_incrementally --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
