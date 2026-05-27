# W183. Runtime Worker Barrier Narrowing

Wave: `W183-runtime-worker-barrier-narrowing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reserve the state/runtime consistency barrier for true state writes and let the
Postgres-backed runtime workers revalidate durable state instead of always
taking the coarse read barrier.

## Feature paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W183-S01` | done | move state registrations back to the state lane and run Postgres-backed collection drain work through the relaxed runtime path after durable revalidation | `cargo test -p venom-api postgres_worker_loop_drains_until_idle --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
