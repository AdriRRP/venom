# W108. Local Scheduler Materialization Veracity

Wave: `W108-local-scheduler-materialization-veracity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep local collection scheduling side-effect free until durable state confirms
the materialization, so the scheduler no longer pretends that one release
schedule already advanced before the durable write succeeds.

## Feature paths

- `features/schedule-collection-scan.feature`
- `features/request-collection-scan.feature`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/scanning/collection_scan_scheduler.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W108-S01` | done | make local due-scan planning pure and derive pending-due state from durable inventory after successful materialization | `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I2`, `I8`, `I11`

## ADR impact

`none`
