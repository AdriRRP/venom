# W115. Contextual Risk Explainability

Wave: `W115-contextual-risk-explainability`
Status: `done`
BDD impact: `update`
Agentic impact: `none`
Infra profile: `none`

## Goal

Expose the deterministic contextual posture behind each contextual risk result.

## Owned paths

- `crates/venom-domain/src/findings/contextual_risk.rs`
- `apps/api/src/app/service.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/findings.tsx`
- `features/classify-finding.feature`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W115-S01` | done | surface contextual posture in domain projections, API DTOs, and operator UI | `unit`, `acceptance`, `web` |

## Invariant impact

`I2`, `I11`
