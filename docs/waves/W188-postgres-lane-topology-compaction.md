# W188. Postgres Lane Topology Compaction

Wave: `W188-postgres-lane-topology-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reduce `ApiState` residency cost by collapsing runtime and publication onto one
shared volatile lane and reusing the same topology in local mode.

## Feature paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W188-S01` | done | collapse runtime and publication into one volatile service slot for Postgres-backed API state | `cargo test -p venom-api postgres_open_shares_bootstrap_snapshot_arcs_across_lanes --all-features --offline` |
| `W188-S02` | done | mirror the same two-lane topology for local `ApiState` opens | `cargo test -p venom-api runtime_and_publication_lanes_do_not_take_the_state_consistency_barrier --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
