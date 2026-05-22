# W105. HTTP Worker Lock Splitting

Wave: `W105-http-worker-lock-splitting`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Stop holding the HTTP-plane service mutex across awaited worker and mutation
operations by moving `ApiApplication` through an explicit service slot, so long
running worker drains no longer retain the global lock while they await real
work.

## Feature paths

- `features/request-scan.feature`
- `features/request-collection-scan.feature`
- `features/publish-integration-events.feature`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W105-S01` | done | replace direct `Mutex<ApiApplication>` locking in HTTP handlers with a service-slot mutation helper that releases the lock before awaited work runs | `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
