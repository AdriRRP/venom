# W125. Context Factor Explainability

Wave: `W125-context-factor-explainability`
Status: `done`
BDD impact: `classify-finding.feature`
Agentic impact: `none`
Infra profile: `none`

## Goal

Expose the exact effective context factors that shaped each deterministic
contextual-risk result so operators can audit the decision path.

## Feature paths

- `features/classify-finding.feature`

## Execution lanes

- `acceptance`
- `unit`
- `web`

## Owned paths

- `crates/venom-domain/src/findings/contextual_risk.rs`
- `apps/api/src/app/service.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/findings.tsx`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W125-S01` | done | expose exact merged context factors through domain projections, API DTOs, and findings UI | `acceptance`, `unit`, `web` |

## Language impact

- none

## Invariant impact

`I9`, `I11`

## ADR impact

`none`
