# W138. HTTP Write-Plane Partitioning

Wave: `W138-http-write-plane-partitioning`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let Postgres-backed fresh reads prove that durable state is unchanged before
they contend on the mutable HTTP application slot.

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W138-S01` | done | add one Postgres remote-change probe shared with the write plane so fresh reads skip the mutable slot unless remote WAL really advanced | `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`

## ADR impact

`none`
