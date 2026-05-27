# W185. Postgres Read Model Remote Delta Tightening

Wave: `W185-postgres-read-model-remote-delta-tightening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Narrow detached Postgres read-model refresh so governance-only and
findings-only remote changes reuse as much unaffected in-memory findings state
as possible.

## Feature paths

- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W185-S01` | done | split provider-report and governance detached refresh paths so the remote findings lane advances from the visible provider-report watermark instead of replaying the whole reports history | `cargo test -p venom-api detached_postgres_read_snapshot_advances_read_model_source_watermark_for_new_reports --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
