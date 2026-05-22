# W121. HTTP Mutation Multi Lane

Wave: `W121-http-mutation-multi-lane`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Move operator-facing `system events` reads fully onto the HTTP read snapshot so
they no longer contend on the mutable application slot.

## Feature paths

- `apps/api/src/http/mod.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W121-S01` | done | serve `/system-events` from indexed read snapshots instead of mutable service inspection | `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
