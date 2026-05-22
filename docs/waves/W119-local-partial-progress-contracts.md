# W119. Local Partial Progress Contracts

Wave: `W119-local-partial-progress-contracts`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Expose partial durable progress explicitly when the local collection scheduler
has already enqueued scan commands but fails before recording schedule
materialization metadata.

## Owned paths

- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/operations.tsx`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W119-S01` | done | extend scheduler drain responses with `partial_progress` and `last_error`, and surface that contract through API and UI without hiding successful durable enqueue work | `integration`, `web` |

## Invariant impact

`I2`, `I3`, `I11`
