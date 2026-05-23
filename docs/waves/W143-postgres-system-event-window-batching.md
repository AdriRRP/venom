# W143. Postgres System Event Window Batching

Wave: `W143-postgres-system-event-window-batching`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce Postgres rebuild chatter for recent operator event windows by loading the
global and per-category recent windows from one ranked query, while keeping
truthful totals separate.

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W143-S01` | done | replace one recent-events query per category with one ranked recent-window query plus totals | `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`

## ADR impact

`none`
