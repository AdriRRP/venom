# W207. System Event Window Cache Shaping

Wave: `W207-system-event-window-cache-shaping`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Cache recent operator event windows inside the truthful query index so category
queries and local merges stop re-filtering the retained set on every refresh.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W207-S01` | done | keep cached recent windows per index and reuse them in category queries and local merges | `cargo test -p venom-domain system_event_query_index_reuses_cached_recent_windows --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`
