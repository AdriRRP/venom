# W95. Local Scan Outcome Atomicity

Wave: `W95-local-scan-outcome-atomicity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Make the local scan worker recoverable across split durable writes by capturing
one successful provider report durably before applying it, and by making the
durable state reuse the same command-scoped report application on retry.

## Feature paths

- `features/request-scan.feature`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W95-S01` | done | make provider report application idempotent by `command_id` inside the durable state | `unit` |
| `W95-S02` | done | introduce one recoverable `applying` phase in the local scan queue and finalize it without rescanning | `integration` |

## Language impact

- `applying`

## Invariant impact

`I2`, `I3`, `I9`

## ADR impact

`none`
