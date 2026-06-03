# W220. Postgres Governance Direct Identity Joins

Wave: `W220-postgres-governance-direct-identity-joins`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Use `governance_journal_id` from the durable governance identity journal to
refresh changed findings directly instead of re-windowing the whole governance
journal per identity.

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
| `W220-S01` | done | switch governance delta refresh to direct identity-journal joins | `cargo test -p venom-api postgres_governance_delta_refresh_uses_direct_identity_joins --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
