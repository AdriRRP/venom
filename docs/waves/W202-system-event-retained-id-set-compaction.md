# W202. System Event Retained Id Set Compaction

Wave: `W202-system-event-retained-id-set-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Tighten the residual memory shape of `SystemEventQueryIndex` by tracking one
dedupe id set instead of a slot map while preserving truthful recent category
pages from one retained newest-first event store.

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
| `W202-S01` | done | replace retained slot topology with one dedupe id set over the retained event store | `cargo test -p venom-domain system_event_query_index_keeps_truthful_recent_category_pages_without_category_slot_topology --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
