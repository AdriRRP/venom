# W90. API Read Snapshot Compaction

Wave: `W90-api-read-snapshot-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reduce API read-path recomputation by caching a compact release-board projection
inside the read snapshot and serving release board and dashboard views from that
projection instead of recomputing collection health over every request.

## Feature paths

- `apps/web/src/routes/operations.tsx`
- `apps/web/src/routes/findings.tsx`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W90-S01` | done | add one compact cached release-board projection to the API read snapshot | `unit` |
| `W90-S02` | done | serve collection board and release dashboard from the cached projection | `unit`, `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
