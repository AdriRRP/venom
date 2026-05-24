# W147. Postgres System Event Arc Window Sharing

Wave: `W147-postgres-system-event-arc-window-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce duplicated recent-event ownership during Postgres rebuild by sharing one
`Arc<SystemEvent>` across the global and per-category recent windows.

## Execution lanes

- `integration`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W147-S01` | done | represent Postgres recent event windows with shared arcs and preserve truthful query semantics | `integration` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`
