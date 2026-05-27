# W176. Shared Postgres Pool Across API Lanes

Wave: `W176-shared-postgres-pool-across-api-lanes`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reuse one `PgPool` across the partitioned Postgres-backed API lanes so one
`ApiState` no longer opens three independent pools for the same schema.

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
| `W176-S01` | done | open one shared Postgres pool and reuse it across `state`, `runtime`, and `publication` lane services | `cargo test -p venom-api detached_postgres_fresh_read_promotes_the_observed_change_watermark --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
