# W29. Hot-Path Performance Baseline

Wave: `W29-hot-path-performance-baseline`
Status: `active`
BDD impact: `none`
Agentic impact: `script`
Infra profile: `none`

## Goal

Establish a repeatable hot-path performance baseline for the current domain core, then apply one measured low-risk optimization in a hot operator query path without changing observable behavior.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/Cargo.toml`
- `crates/venom-domain/benches/**`
- `crates/venom-domain/src/findings/finding_read_model.rs`
- `scripts/check-performance-baseline.sh`
- `scripts/README.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W29-S01` | done | define the performance baseline wave, scope, and verification contract | `./scripts/check-slice.sh --wave W29-hot-path-performance-baseline --slice W29-S01 --path docs/waves/ACTIVE --path docs/waves/W29-hot-path-performance-baseline.md` |
| `W29-S02` | done | add a deterministic hot-path benchmark harness and benchmark script | `./scripts/check-performance-baseline.sh` |
| `W29-S03` | in_progress | remove unnecessary cloning from the active findings query hot path and keep benchmark coverage green | `cargo test --workspace --all-targets --all-features && ./scripts/check-performance-baseline.sh` |
| `W29-S04` | planned | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W29-hot-path-performance-baseline` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- benchmark numbers are local guidance, not committed golden outputs
- the first performance wave should optimize only where a benchmark exists in the same wave
