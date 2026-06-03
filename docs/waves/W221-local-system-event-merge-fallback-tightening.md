# W221. Local System Event Merge Fallback Tightening

Wave: `W221-local-system-event-merge-fallback-tightening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep the local merged system-event cache on incremental paths for more bounded
single-side changes before falling back to full recomposition.

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
| `W221-S01` | done | reuse cached merged windows across wider bounded single-side churn | `cargo test -p venom-api local_merged_system_event_snapshot_reuses_cached_windows_across_bounded_single_side_churn --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`
