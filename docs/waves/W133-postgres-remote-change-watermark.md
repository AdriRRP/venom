# W133. Postgres Remote Change Watermark

Wave: `W133-postgres-remote-change-watermark`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `postgres`

## Goal

Stop trusting one Postgres-backed process cache forever by refreshing API
snapshots when the durable store advanced in another instance.

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W133-S01` | done | use one Postgres remote change watermark to refresh stale read snapshots before serving reads | `integration` |

## Language impact

`none`

## Invariant impact

`I5`, `I9`, `I11`
