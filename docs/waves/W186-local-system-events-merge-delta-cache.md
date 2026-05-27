# W186. Local System Events Merge Delta Cache

Wave: `W186-local-system-events-merge-delta-cache`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce local system-event merge churn by reusing one incrementally refreshed
merged snapshot when only one source lane advanced.

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
| `W186-S01` | done | refresh merged local system-event windows from cached peer windows when only one source side changed instead of re-extracting both recent windows every time | `cargo test -p venom-api local_merged_system_event_snapshot_reuses_cached_peer_window --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
