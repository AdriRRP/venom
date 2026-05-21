# W99. Contextual Risk Semantic Completion

Wave: `W99-contextual-risk-semantic-completion`
Status: `done`
BDD impact: `classify-finding.feature`
Agentic impact: `none`
Infra profile: `none`

## Goal

Make the richer execution-context model affect deterministic contextual risk for
operator-facing findings instead of stopping at `internet_exposed`,
`production`, and `mission_critical`.

## Feature paths

- `features/classify-finding.feature`
- `features/manage-context-profiles.feature`

## Execution lanes

- `acceptance`
- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/findings/contextual_risk.rs`
- `crates/venom-domain/examples/acceptance.rs`
- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W99-S01` | done | make `vpn_restricted` and `non_privileged_user` affect deterministic contextual risk and lock that through BDD, domain tests, and API tests | `acceptance`, `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I2`, `I11`

## ADR impact

`none`
