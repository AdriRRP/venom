# W89. System Event Trace Veracity

Wave: `W89-system-event-trace-veracity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Make operator-facing system events truthful and backend-consistent: `total`
must mean total matching events rather than returned events, and local and
Postgres paths must expose the same bounded recent-event window.

## Feature paths

- `apps/web/src/routes/events.tsx`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W89-S01` | done | return truthful totals for recent system events queries | `unit` |
| `W89-S02` | done | keep the recent-event window consistent across local and Postgres backends | `unit`, `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
