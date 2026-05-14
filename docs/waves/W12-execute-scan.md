# W12. Execute Scan

Wave: `W12-execute-scan`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `none`

## Goal

Execute canonical scan requests through a finding provider and apply the resulting provider snapshot to managed ownership.

## Feature paths

- `features/request-scan.feature`

## Execution lanes

- `unit`
- `acceptance`

## Owned paths

- `docs/ubiquitous-language.md`
- `docs/waves/W12-execute-scan.md`
- `features/request-scan.feature`
- `crates/venom-domain/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W12-S01` | done | add a minimal scan execution service that runs one canonical scan request through a provider and applies the provider snapshot to managed ownership | `./scripts/check-acceptance.sh`, `cargo test --workspace --all-targets --all-features`, `./scripts/check-wave.sh --wave W12-execute-scan` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- execution must stay provider-agnostic at the domain level
- scheduling, retries, and durable command flow come later
