# W34. Live Provider Byte Path

Wave: `W34-live-provider-byte-path`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `full`

## Goal

Reduce live provider execution allocations by parsing Syft and Grype JSON directly from process output bytes instead of materializing intermediate UTF-8 strings first.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `crates/venom-domain/src/scanning/syft_grype.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W34-S01` | in_progress | define the live provider byte path wave and target | `./scripts/check-slice.sh --wave W34-live-provider-byte-path --slice W34-S01 --path docs/waves/ACTIVE --path docs/waves/W34-live-provider-byte-path.md` |
| `W34-S02` | planned | parse live Syft and Grype output directly from bytes and keep fixture behavior unchanged | `cargo test --workspace --all-targets --all-features && ./scripts/rehearse-infra.sh --profile full` |
| `W34-S03` | planned | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W34-live-provider-byte-path` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- optimize the live path only
- preserve timeout, error, and contract semantics
