# W11. Request Scan

Wave: `W11-request-scan`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `none`

## Goal

Create canonical scan requests only for managed components and owned immutable artifacts.

## Feature paths

- `features/request-scan.feature`

## Execution lanes

- `unit`
- `acceptance`

## Owned paths

- `docs/product-direction.md`
- `docs/ubiquitous-language.md`
- `docs/waves/W11-request-scan.md`
- `features/request-scan.feature`
- `crates/venom-domain/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W11-S01` | done | add a minimal scan planner that produces canonical scan requests from managed ownership and expose the behavior through acceptance BDD | `./scripts/check-acceptance.sh`, `cargo test --workspace --all-targets --all-features`, `./scripts/check-wave.sh --wave W11-request-scan` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- scan planning must stay domain-level and provider-agnostic
- execution, scheduling, and retries come later
