# W17. Scan Request Api And Command State

Wave: `W17-scan-request-api-and-command-state`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Expose durable scan request enqueueing and command-status queries through the application API without coupling the app to provider execution details.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/**`
- `docs/waves/W17-scan-request-api-and-command-state.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W17-S01` | done | add API support to enqueue canonical scan requests and query durable command status | `cargo test --workspace --all-targets --all-features`, `./scripts/check-wave.sh --wave W17-scan-request-api-and-command-state` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- provider execution remains outside the app API for now
