# W150. Schema Scoped Postgres Remote Refresh

Wave: `W150-schema-scoped-postgres-remote-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Detect remote Postgres changes from one VENOM schema-local watermark instead of
the database-global WAL head.

## Feature paths

- `none`

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W150-S01` | done | install one schema-local change watermark and drive remote probes from it | `cargo test -p venom-api postgres_backend --all-features` |

## Language impact

`none`

## Invariant impact

`I8, I9`

## ADR impact

`none`

