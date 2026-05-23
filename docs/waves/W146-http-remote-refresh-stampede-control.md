# W146. HTTP Remote Refresh Stampede Control

Wave: `W146-http-remote-refresh-stampede-control`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Prevent concurrent Postgres-backed fresh reads from stampeding the mutable HTTP
application slot when one remote refresh is enough to update the shared
snapshot.

## Execution lanes

- `unit`

## Owned paths

- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W146-S01` | done | gate one stale-read remote refresh at a time and recheck freshness before taking the service slot | `unit` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`

## ADR impact

`none`
