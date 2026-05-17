# W50. Inventory Clone Reduction

Wave: `W50-inventory-clone-reduction`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reduce unnecessary full-ingestion clones on inventory-only durable writes in both local and Postgres backends, keeping the code explicit and reversible while cutting memory churn on common operator mutations.

## Feature paths

- `features/register-component.feature`
- `features/manage-collections.feature`
- `features/schedule-collection-scan.feature`

## Execution lanes

- `unit`
- `integration`
- `infra`
- `acceptance`

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/waves/W50-inventory-clone-reduction.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W50-S01` | done | replace full ingestion clones with inventory-only clones on inventory-only writes | `scripts/check-slice.sh --wave W50-inventory-clone-reduction --slice W50-S01 --lane integration --path crates/venom-domain/src/durable_state.rs --path apps/api/src/infra/postgres_backend.rs` |
| `W50-S02` | planned | close the wave with docs and full gate alignment | `scripts/check-wave.sh --wave W50-inventory-clone-reduction` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- prefer narrower clones over speculative preview abstractions
- do not introduce hidden rollback logic or partial durable writes
- keep tracker and read-model cloning only where those structures are actually touched
