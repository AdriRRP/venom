# W30. API Lock Contention Reduction

Wave: `W30-api-lock-contention-reduction`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce avoidable API-side lock contention by separating read-only request paths from mutating request paths, while preserving the current application semantics and durability behavior.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W30-S01` | done | define the lock-contention reduction wave and target | `./scripts/check-slice.sh --wave W30-api-lock-contention-reduction --slice W30-S01 --path docs/waves/ACTIVE --path docs/waves/W30-api-lock-contention-reduction.md` |
| `W30-S02` | done | replace the global API mutex with a read-write lock and route read-only handlers through shared reads | `cargo test --workspace --all-targets --all-features` |
| `W30-S03` | in_progress | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W30-api-lock-contention-reduction` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- this wave reduces contention only at the API state lock layer
- it does not yet redesign backend ownership or worker execution topology
