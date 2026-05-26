# W164. Inventory Source Arc Sharing

Wave: `W164-inventory-source-arc-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Eliminate deep inventory clones on read-side snapshot refresh by keeping the
live inventory source under shared `Arc` ownership and using copy-on-write only
when one mutation actually changes it.

## Feature paths

- `crates/venom-domain/src/findings/finding_ingestion.rs`
- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/findings/finding_ingestion.rs`
- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W164-S01` | done | keep the live inventory under `Arc<ComponentInventory>` ownership and refresh snapshot lanes by cloning the source `Arc` instead of cloning the whole inventory | `cargo test -p venom-domain inventory_snapshot_cache_reuses_live_inventory_arc --all-features` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`

## Notes

This wave narrows one of the biggest remaining read-side copy hotspots without
changing observable business semantics.
