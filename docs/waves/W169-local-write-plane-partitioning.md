# W169. Local Write Plane Partitioning

Wave: `W169-local-write-plane-partitioning`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove the local HTTP profile from the last global mutable single-flight path
without sacrificing durable correctness in the file-backed state and runtime
stores.

## Feature paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W169-S01` | done | make local history appends safe for partitioned lane handles | `cargo test -p venom-domain durable_state --all-features --offline` |
| `W169-S02` | done | partition local HTTP lanes with local-change refresh coordination before mutation | `cargo test -p venom-api api_requests_collection_scan_batch_for_multiple_members --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
