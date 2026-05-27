# W174. Postgres Sub-Lane Refresh Narrowing

Wave: `W174-postgres-sub-lane-refresh-narrowing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Narrow detached Postgres refreshes so common remote changes rebuild only the
inventory or findings subgraph that actually changed instead of one broad lane.

## Feature paths

- `apps/api/src/infra/postgres_backend.rs`
- `crates/venom-domain/src/inventory/component_inventory.rs`
- `crates/venom-domain/src/findings/finding_read_model.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `crates/venom-domain/src/inventory/component_inventory.rs`
- `crates/venom-domain/src/findings/finding_read_model.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W174-S01` | done | split inventory refresh into narrower detached subgraphs that reuse current shared `Arc` sources | `cargo test -p venom-api --all-features --offline -- --nocapture` |
| `W174-S02` | done | split read-model refresh into provider-report and governance detached subgraphs | `cargo test -p venom-api --all-features --offline -- --nocapture` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
