# W24. Active Findings Query Model

Wave: `W24-active-findings-query-model`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `db`

## Goal

Make active findings queries more operationally useful with stable ordering, lightweight filtering, and bounded paging.

## Feature paths

- `features/view-active-findings.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `infra`

## Owned paths

- `features/view-active-findings.feature`
- `crates/venom-domain/**`
- `apps/api/**`
- `docs/waves/W24-active-findings-query-model.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W24-S01` | done | add a canonical active findings query with stable ordering, severity/package filters, and bounded page limits, then expose it through the API and acceptance coverage | `cargo test --workspace --all-targets --all-features`, `./scripts/check-acceptance.sh`, `./scripts/rehearse-infra.sh --profile db`, `./scripts/check-wave.sh --wave W24-active-findings-query-model` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- query behavior must stay deterministic
- page limits must be explicit and bounded
