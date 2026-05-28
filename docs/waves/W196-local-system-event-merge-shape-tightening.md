# W196. Local System Event Merge Shape Tightening

Wave: `W196-local-system-event-merge-shape-tightening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Tighten the local merged system-event cache so bounded appends on one side reuse
the retained event shape directly instead of rebuilding merged recent windows
wholesale.

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
| `W196-S01` | done | merge bounded local deltas directly over retained event vectors instead of rebuilding merged recent windows from scratch | `cargo test -p venom-api local_merged_system_event_snapshot_appends_bounded_delta_without_window_rebuild --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
