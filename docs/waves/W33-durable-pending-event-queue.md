# W33. Durable Pending Event Queue

Wave: `W33-durable-pending-event-queue`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce local durable replay and publication cost by storing pending integration events as one FIFO queue instead of a vector that requires linear front-removal work.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W33-S01` | done | define the durable pending event queue wave and target | `./scripts/check-slice.sh --wave W33-durable-pending-event-queue --slice W33-S01 --path docs/waves/ACTIVE --path docs/waves/W33-durable-pending-event-queue.md` |
| `W33-S02` | done | replace local pending integration event vectors with FIFO queues and explicit front-removal semantics | `cargo test --workspace --all-targets --all-features` |
| `W33-S03` | done | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W33-durable-pending-event-queue` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- preserve publication ordering
- keep explicit replay semantics
- do not add hidden compaction or background cleanup
