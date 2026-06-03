# W222. System Event Index Ring Window Shape

Wave: `W222-system-event-index-ring-window-shape`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Replace vector front-rotation in the recent-event hot path with ring-style
storage so append and trim work stay cheap while preserving truthful bounded
recent windows.

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
| `W222-S01` | done | switch recent-window push path to ring-style storage without changing query semantics | `cargo test -p venom-domain system_event_query_index_push_uses_ring_windows_without_semantic_regression --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`
