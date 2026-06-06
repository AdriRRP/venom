# W228. System Event Cache Shape Closure

Wave: `W228-system-event-cache-shape-closure`
Status: `done`
BDD impact: `none`
Agentic impact: `compact`
Infra profile: `none`

## Goal

Close the remaining hot-path `system events` shape debt by introducing one
cache-oriented recent-window shape in the domain and making the local merged
snapshot path reuse that shape directly instead of round-tripping through
public `Vec` windows.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `crates/venom-domain/src/operations/mod.rs`
- `crates/venom-domain/src/lib.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W228-S01` | done | add one cache-native recent-window shape to the domain and derive public windows from it only at the boundary | `cargo test -p venom-domain system_event_query_index_merged_with_recent_windows_reuses_one_window_build --all-features --offline` |
| `W228-S02` | done | switch local merged `system events` cache reuse onto the cache-native window shape | `cargo test -p venom-api local_merged_system_event_snapshot_reuses_domain_merged_windows --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
