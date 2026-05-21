# W96. Local Collection Scan Request Atomicity

Wave: `W96-local-collection-scan-request-atomicity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove the local per-command append loop from collection scan requests so one
operator batch is durably enqueued through one queue write instead of a partial
series of independent appends.

## Feature paths

- `features/request-collection-scan.feature`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W96-S01` | done | add one durable batch enqueue contract to the local scan queue | `unit` |
| `W96-S02` | done | route local collection scan requests through the batch path and keep the observable API contract truthful | `integration` |

## Language impact

- `batch enqueued`

## Invariant impact

`I2`, `I3`, `I9`

## ADR impact

`none`
