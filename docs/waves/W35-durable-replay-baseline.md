# W35. Durable Replay Baseline

Wave: `W35-durable-replay-baseline`
Status: `active`
BDD impact: `none`
Agentic impact: `script`
Infra profile: `none`

## Goal

Establish a repeatable replay baseline for local durable history rebuild paths, then remove one avoidable allocation pattern from local integration-event publication without changing observable behavior.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/benches/**`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W35-S01` | done | define the durable replay baseline wave and target | `./scripts/check-slice.sh --wave W35-durable-replay-baseline --slice W35-S01 --path docs/waves/ACTIVE --path docs/waves/W35-durable-replay-baseline.md` |
| `W35-S02` | done | add deterministic durable replay benchmarks for local state and runtime rebuild paths | `./scripts/check-performance-baseline.sh` |
| `W35-S03` | in_progress | remove avoidable publication batch cloning from local durable pending-event paths and keep baseline green | `cargo test --workspace --all-targets --all-features && ./scripts/check-performance-baseline.sh` |
| `W35-S04` | in_progress | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W35-durable-replay-baseline` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- replay baseline numbers are local guidance, not golden assertions
- optimize only paths that the same wave benchmarks or directly exercises
