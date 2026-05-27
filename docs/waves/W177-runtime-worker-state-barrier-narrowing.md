# W177. Runtime Worker State Barrier Narrowing

Wave: `W177-runtime-worker-state-barrier-narrowing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let the scan-command worker lane avoid the coarse state read barrier where it
can revalidate to the latest durable state immediately before applying command
outcomes.

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
| `W177-S01` | done | classify runtime worker execution as state-independent at HTTP lane level once the backend revalidates durable state before applying outcomes | `cargo test -p venom-api postgres_worker_loop_drains_until_idle --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
