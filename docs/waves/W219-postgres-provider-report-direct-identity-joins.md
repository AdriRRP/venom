# W219. Postgres Provider Report Direct Identity Joins

Wave: `W219-postgres-provider-report-direct-identity-joins`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Use `provider_report_id` from the durable identity journal to refresh changed
provider reports directly instead of paying a `ROW_NUMBER()` partition over the
whole provider-report table for every changed artifact identity.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W219-S01` | done | switch provider-report delta refresh to direct identity-journal joins | `cargo test -p venom-api postgres_provider_report_delta_refresh_uses_direct_identity_joins --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
