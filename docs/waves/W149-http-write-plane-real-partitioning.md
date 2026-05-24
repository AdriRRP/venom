# W149. HTTP Write Plane Real Partitioning

Wave: `W149-http-write-plane-real-partitioning`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Let stale Postgres-backed fresh reads rebuild one detached read snapshot without
taking the live mutable application slot.

## Feature paths

- `none`

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W149-S01` | done | load stale Postgres-backed fresh reads through one detached snapshot loader instead of the live write slot | `cargo test -p venom-api postgres_backend --all-features` |

## Language impact

`none`

## Invariant impact

`I8, I11`

## ADR impact

`none`
