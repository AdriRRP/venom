# W171. System Event Merge Cost Compaction

Wave: `W171-system-event-merge-cost-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep merged local system-event snapshots bounded and truthful without
reconstructing merged recent windows through repeated query passes.

## Feature paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `apps/api/src/app/service.rs`

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W171-S01` | done | merge recent windows directly from retained ids and shared arcs | `cargo test -p venom-domain system_event_trace --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`
