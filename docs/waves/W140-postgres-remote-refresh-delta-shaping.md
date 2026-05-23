# W140. Postgres Remote Refresh Delta Shaping

Wave: `W140-postgres-remote-refresh-delta-shaping`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Avoid recomputing the local merged `system events` lane when its state and
runtime source arcs did not change.

## Execution lanes

- `unit`

## Owned paths

- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W140-S01` | done | memoize the merged local `system events` snapshot by source-arc identity so HTTP reads and snapshot refreshes reuse it until one side really changes | `unit` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
