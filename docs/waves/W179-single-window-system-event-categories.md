# W179. Single-Window System Event Categories

Wave: `W179-single-window-system-event-categories`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Derive category-scoped recent system-event views from one shared recent window
instead of storing separate recent slot lists per category.

## Feature paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W179-S01` | done | keep one recent event window and derive category views from it during queries and detached rebuilds | `cargo test -p venom-domain system_events_query_reports_total_matches_not_only_returned_events --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
