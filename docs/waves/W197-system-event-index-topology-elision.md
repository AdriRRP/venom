# W197. System Event Index Topology Elision

Wave: `W197-system-event-index-topology-elision`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove dedicated category-slot topology from `SystemEventQueryIndex` while
preserving truthful recent category pages from one shared retained event store.

## Feature paths

- `crates/venom-domain/src/operations/system_event_trace.rs`

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W197-S01` | done | answer truthful recent category pages from one retained newest-first event store without per-category slot topology | `cargo test -p venom-domain system_event_query_index_keeps_truthful_recent_category_pages_without_category_slot_topology --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
