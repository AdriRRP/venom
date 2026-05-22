# W109. Contextual Risk Model Enrichment

Wave: `W109-contextual-risk-model-enrichment`
Status: `done`
BDD impact: `update`
Agentic impact: `none`
Infra profile: `none`

## Goal

Replace the flat contextual-risk sum with deterministic postures that better
match operator meaning.

## Feature paths

- `features/classify-finding.feature`

## Execution lanes

- `unit`
- `acceptance`

## Owned paths

- `crates/venom-domain/src/findings/contextual_risk.rs`
- `crates/venom-domain/examples/acceptance.rs`
- `features/classify-finding.feature`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W109-S01` | done | distinguish public critical, critical internal, and hardened private workloads with deterministic posture rules | `unit`, `acceptance` |

## Language impact

- none

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
