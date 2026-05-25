# W159. System Event Page Arc Sharing

Wave: `W159-system-event-page-arc-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Avoid one avoidable full `SystemEvent` clone per indexed system-event query by
keeping the query page on shared `Arc<SystemEvent>` entries until API DTO
projection.

## Feature paths

- `none`

## Execution lanes

- `integration`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W159-S01` | done | keep indexed system-event pages on shared arcs until API projection and preserve current query semantics | `./scripts/check-wave.sh --wave W159-system-event-page-arc-sharing` |

## Language impact

`none`

## Invariant impact

`I8, I11`

## ADR impact

`none`
