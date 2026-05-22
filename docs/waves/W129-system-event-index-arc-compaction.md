# W129. System Event Index Arc Compaction

Wave: `W129-system-event-index-arc-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce duplicated in-memory payloads inside the bounded system-event query index
without changing operator-visible query semantics.

## Owned paths

- `crates/venom-domain/src/operations/system_event_trace.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W129-S01` | done | store retained system events once and reuse shared references across global and category windows | `unit` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`
