# W92. Gate Signal Hardening

Wave: `W92-gate-signal-hardening`
Status: `done`
BDD impact: `none`
Agentic impact: `script`
Infra profile: `db`

## Goal

Replace the remaining format-coupled gate signals with direct failure signals
from the underlying tools where practical, and preserve useful diagnostics when
browser smoke fails.

## Feature paths

- `features/**`

## Execution lanes

- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/examples/acceptance.rs`
- `scripts/check-acceptance.sh`
- `scripts/check-web-e2e.sh`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W92-S01` | done | make acceptance fail on skipped coverage from the runner itself instead of grepping terminal output | `acceptance` |
| `W92-S02` | done | preserve browser smoke diagnostics and keep sandbox failures explicit | `e2e` |

## Language impact

`none`

## Invariant impact

`I2`, `I9`

## ADR impact

`none`
