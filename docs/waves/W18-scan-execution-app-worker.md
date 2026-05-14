# W18. Scan Execution App Worker

Wave: `W18-scan-execution-app-worker`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Execute the next durable scan command through the app layer using an injected canonical provider input, while keeping the app provider-agnostic.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/**`
- `docs/waves/W18-scan-execution-app-worker.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W18-S01` | done | add an app worker endpoint that runs the next queued scan with an injected canonical provider response or provider failure | `cargo test --workspace --all-targets --all-features`, `./scripts/check-wave.sh --wave W18-scan-execution-app-worker` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- provider execution input stays canonical and transport-facing
- real provider daemons can replace the fixture-style execution input in later waves
