# W114. Postgres Bulk Governance Batching

Wave: `W114-postgres-bulk-governance-batching`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Batch Postgres bulk-governance upserts for risk acceptance and suppression.

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W114-S01` | done | replace one-row-at-a-time bulk upserts with batched query-builder writes | `integration` |

## Invariant impact

`I8`, `I11`
