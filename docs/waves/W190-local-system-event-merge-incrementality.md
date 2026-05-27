# W190. Local System Event Merge Incrementality

Wave: `W190-local-system-event-merge-incrementality`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Avoid rebuilding merged local system-event windows from scratch when one side
only advanced by a bounded append delta.

## Feature paths

- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`

## Execution lanes

- `unit`

## Owned paths

- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W190-S01` | done | detect bounded append deltas on one side of a merged local system-event cache and merge only the delta window | `cargo test -p venom-api local_merged_system_event_snapshot_reuses_cached_peer_window --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
