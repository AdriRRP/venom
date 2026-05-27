# W181. Postgres Lane Bootstrap Forking

Wave: `W181-postgres-lane-bootstrap-forking`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Pay one Postgres rebuild at `ApiState` open time and fork the remaining API
lanes from that bootstrapped state instead of rebuilding all three.

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
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W181-S01` | done | open one rebuilt Postgres store, fork runtime/publication lanes from that base, and share initial snapshot arcs across the lane services | `cargo test -p venom-api postgres_open_shares_bootstrap_snapshot_arcs_across_lanes --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
