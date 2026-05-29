# W214. Read Model Identity Journal Refresh

Wave: `W214-read-model-identity-journal-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Refresh Postgres findings and governance read-model lanes from compact identity
journals so hot refreshes stop scanning broad watermark ranges on source
tables.

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
| `W214-S01` | done | drive provider-report and governance refresh through durable changed-identity journals | `cargo test -p venom-api postgres_read_model_refresh_uses_identity_journal_cursors --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
