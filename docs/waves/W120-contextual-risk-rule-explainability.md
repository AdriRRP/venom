# W120. Contextual Risk Rule Explainability

Wave: `W120-contextual-risk-rule-explainability`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Expose the deterministic contextual-risk rule that produced each projected
operator risk result.

## Owned paths

- `crates/venom-domain/src/findings/contextual_risk.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/findings.tsx`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W120-S01` | done | project the applied contextual-risk rule through domain, API, HTTP tests, and the findings UI so operator prioritization stays explainable | `unit`, `integration`, `web` |

## Invariant impact

`I2`, `I11`
