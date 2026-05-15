# W20. Scan Execution Worker Loop

Wave: `W20-scan-execution-worker-loop`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let the app drain durable pending scan commands until idle through one bounded worker loop, while preserving explicit terminal command state.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `apps/api/**`
- `scripts/infra-smoke.sh`
- `docs/waves/W20-scan-execution-worker-loop.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W20-S01` | done | add one bounded app worker loop that drains pending scan commands until idle and proves the behavior through API and db rehearsal | `./scripts/check-quality.sh`, `cargo test --workspace --all-targets --all-features`, `./scripts/rehearse-infra.sh --profile db`, `./scripts/check-wave.sh --wave W20-scan-execution-worker-loop` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- worker execution remains provider-agnostic and uses the canonical provider boundary
- the loop must stay bounded by an explicit command limit
