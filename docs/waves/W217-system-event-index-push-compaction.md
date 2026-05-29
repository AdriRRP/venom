# W217. System Event Index Push Compaction

Wave: `W217-system-event-index-push-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Compact `SystemEventQueryIndex` push-path work so new events stop paying
front-insert plus whole retained-id rebuild cost on every append.

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
| `W217-S01` | done | switch recent windows and retained-id bookkeeping to bounded append/pop semantics | `cargo test -p venom-domain system_event_query_index_push_keeps_recent_windows_without_full_id_rebuilds --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`
