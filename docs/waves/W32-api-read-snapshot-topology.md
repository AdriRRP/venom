# W32. API Read Snapshot Topology

Wave: `W32-api-read-snapshot-topology`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Remove API read traffic from the write lock path by serving read-only endpoints from one refreshed read snapshot instead of borrowing the mutable application service directly.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W32-S01` | done | define the API read snapshot topology wave and target | `./scripts/check-slice.sh --wave W32-api-read-snapshot-topology --slice W32-S01 --path docs/waves/ACTIVE --path docs/waves/W32-api-read-snapshot-topology.md` |
| `W32-S02` | done | move API reads to one refreshed snapshot and reduce write-path lock contention | `cargo test --workspace --all-targets --all-features && ./scripts/rehearse-infra.sh --profile db` |
| `W32-S03` | done | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W32-api-read-snapshot-topology` |

## Language impact

- add `API read snapshot`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep write semantics explicit and single-owner
- optimize contention without introducing hidden background refresh
