# W152. Event Trace Index Shape Compaction

Wave: `W152-event-trace-index-shape-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Merge bounded recent system-event windows directly instead of chain-sort-
truncate rebuilds on every local composite refresh.

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
| `W152-S01` | done | replace bounded recent-event merge sort with one direct linear merge | `cargo test -p venom-domain finding_read_model --all-features` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`

