# W175. System Event Slot Window Compaction

Wave: `W175-system-event-slot-window-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Compact the bounded `system events` index so one retained event body is shared
through slot references across recent windows instead of duplicating ids per
window.

## Feature paths

- `crates/venom-domain/src/operations/system_event_trace.rs`

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W175-S01` | done | replace duplicated recent-window id storage with compact slot references over one retained event store | `cargo test -p venom-domain --all-features --offline system_event -- --nocapture` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
