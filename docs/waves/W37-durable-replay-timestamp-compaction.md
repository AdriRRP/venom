# W37. Durable Replay Timestamp Compaction

Wave: `W37-durable-replay-timestamp-compaction`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce durable replay cost and history size by storing provider report observation time in a compact numeric form instead of reparsing RFC3339 text on every rebuild, while keeping legacy durable history replayable.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/benches/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W37-S01` | in_progress | define the durable replay timestamp compaction wave and target | `./scripts/check-slice.sh --wave W37-durable-replay-timestamp-compaction --slice W37-S01 --path docs/waves/ACTIVE --path docs/waves/W37-durable-replay-timestamp-compaction.md` |
| `W37-S02` | planned | store observed-at values in compact numeric durable form and keep legacy RFC3339 replay compatibility | `cargo test --workspace --all-targets --all-features && ./scripts/check-performance-baseline.sh` |
| `W37-S03` | planned | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W37-durable-replay-timestamp-compaction` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep durable history backward-compatible for replay
- optimize replay parsing cost before touching broader storage design
