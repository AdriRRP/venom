# W212. Local System Event Merge Tightening

Wave: `W212-local-system-event-merge-tightening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Tighten local merged `system events` refreshes so bounded delta appends avoid
full merged-window rebuilds whenever one side advances cleanly.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W212-S01` | done | keep local merged windows on append-style deltas without rebuilding the whole fused view | `cargo test -p venom-api local_merged_system_event_snapshot_preserves_cached_windows_across_bounded_peer_deltas --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`
