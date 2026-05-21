# W93. Bulk Governance Cohort Veracity

Wave: `W93-bulk-governance-cohort-veracity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Make bulk governance actions operate over their full matched cohort instead of
accidentally reusing paged read queries.

## Feature paths

- `features/bulk-accept-risk.feature`
- `features/bulk-suppress-finding.feature`
- `features/reopen-finding.feature`
- `features/bulk-governance-by-tag.feature`

## Execution lanes

- `unit`
- `integration`
- `db`

## Owned paths

- `crates/venom-domain/src/findings/finding_read_model.rs`
- `crates/venom-domain/src/findings/mod.rs`
- `crates/venom-domain/src/lib.rs`
- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W93-S01` | done | split paged scoped queries from bulk governance cohort queries in the domain and local durable path | `unit` |
| `W93-S02` | done | align API and Postgres bulk operations with the cohort query and prove behavior above the page cap | `integration`, `db` |

## Language impact

`none`

## Invariant impact

`I2`, `I3`, `I11`

## ADR impact

`none`
