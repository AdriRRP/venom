# W211. System Event Index Topology Compaction

Wave: `W211-system-event-index-topology-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Collapse the residual `SystemEventQueryIndex` retained-set duplication so the
truthful event index no longer carries both a global retained vector and recent
windows of the same events.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W211-S01` | done | remove the extra retained-event topology while keeping truthful totals, merge, and delta semantics | `cargo test -p venom-domain system_event_query_index_keeps_truthful_recent_category_pages_without_retained_vector_duplication --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`
