# W199. Provider Report Latest Delta Refresh

Wave: `W199-provider-report-latest-delta-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Keep detached Postgres findings refresh aligned to the latest durable provider
snapshot per artifact instead of replaying every intermediate report row after
the watermark.

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
| `W199-S01` | done | reload detached findings deltas from the latest changed provider report head per artifact | `cargo test -p venom-api detached_postgres_read_snapshot_advances_read_model_source_watermark_for_new_reports --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
