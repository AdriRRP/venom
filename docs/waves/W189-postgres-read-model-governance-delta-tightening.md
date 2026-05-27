# W189. Postgres Read-Model Governance Delta Tightening

Wave: `W189-postgres-read-model-governance-delta-tightening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Tighten detached Postgres findings refresh so remote reads do not widen back to
whole-subgraph reloads where a durable delta is sufficient.

## Feature paths

- `apps/api/src/infra/postgres_backend.rs`
- `apps/api/src/app/service.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W189-S01` | done | keep detached findings refresh aligned to durable source cursors instead of rebuilding unrelated state | `cargo test -p venom-api detached_postgres_read_snapshot_advances_read_model_source_watermark_for_new_reports --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
