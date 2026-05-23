# W136. Postgres Write-Path Remote Refresh

Wave: `W136-postgres-write-path-remote-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Refresh one Postgres-backed API instance from durable state before it evaluates
mutable business writes, so multi-instance governance and scan operations do
not act on stale in-process read models.

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W136-S01` | done | refresh one Postgres-backed write path before the HTTP mutation closure runs, and cover it with a two-instance regression | `integration` |

## Language impact

`none`

## Invariant impact

`I2`, `I9`

## ADR impact

`none`
