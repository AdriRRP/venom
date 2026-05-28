# W201. Local System Event Side Window Reuse

Wave: `W201-local-system-event-side-window-reuse`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reuse the cached recent windows for the unchanged local `system events` side
and merge only the bounded delta from the changed side instead of rebuilding
that side window from the whole source index.

## Feature paths

- `apps/api/src/app/service.rs`

## Execution lanes

- `unit`

## Owned paths

- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W201-S01` | done | merge bounded state/runtime side deltas over cached side windows instead of recomputing them from the full source index | `cargo test -p venom-api local_merged_system_event_snapshot_appends_bounded_delta_without_window_rebuild --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
