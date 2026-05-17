# W55. Domain Semantic Renames

Wave: `W55-domain-semantic-renames`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Apply the first bounded semantic rename in the domain crate by renaming the durable scan runtime vocabulary to the clearer `ScanCommandQueue` family.

## Feature paths

- `features/request-scan.feature`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `crates/venom-domain/src/scanning/mod.rs`
- `crates/venom-domain/src/lib.rs`
- `crates/venom-domain/examples/**`
- `crates/venom-domain/benches/hot_paths.rs`
- `apps/api/src/**`
- `docs/waves/W55-domain-semantic-renames.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W55-S01` | done | rename `DurableScanRuntime` and `DurableScanRuntimeError` to the semantically clearer `ScanCommandQueue` family across domain, app, tests, and docs | `scripts/check-slice.sh --wave W55-domain-semantic-renames --slice W55-S01 --lane integration --path crates/venom-domain/src/scanning/durable_scan_runtime.rs --path crates/venom-domain/src/scanning/mod.rs --path crates/venom-domain/src/lib.rs --path crates/venom-domain/examples --path crates/venom-domain/benches/hot_paths.rs --path apps/api/src` |
| `W55-S02` | planned | close the wave with docs and full gate alignment | `scripts/check-wave.sh --wave W55-domain-semantic-renames` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- prefer role-facing names over mechanism-heavy names like `runtime`
