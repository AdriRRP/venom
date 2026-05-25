# W158. Detached Read Watermark Promotion

Wave: `W158-detached-read-watermark-promotion`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

When one detached Postgres-backed fresh read successfully rebuilds a current
snapshot, advance the shared observed change watermark too so the next live
write path does not redundantly refresh the same remote state.

## Feature paths

- `none`

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W158-S01` | done | promote the shared observed Postgres watermark after detached fresh reads and cover it with one multi-instance regression | `./scripts/check-wave.sh --wave W158-detached-read-watermark-promotion` |

## Language impact

`none`

## Invariant impact

`I8, I9, I11`

## ADR impact

`none`
