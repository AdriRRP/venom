# W134. Postgres System-Event Window Shaping

Wave: `W134-postgres-system-event-window-shaping`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `postgres`

## Goal

Rebuild Postgres-backed operator event timelines from truthful totals and
recent windows instead of loading the whole `system_events` table into memory.

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W134-S01` | done | load total counts plus recent global/category windows when rebuilding the Postgres system-event index | `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`
