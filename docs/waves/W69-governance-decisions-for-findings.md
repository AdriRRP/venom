# W69. Governance decisions for findings

Wave: `W69-governance-decisions-for-findings`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Make active findings governable through one first durable operator decision that
survives reload, stays visible in release-scoped queries, and does not couple
read views back to mutable write state.

## Feature paths

- `features/accept-risk.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/src/findings/**`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/examples/acceptance.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/findings.tsx`
- `apps/web/src/routes/findings.test.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `features/accept-risk.feature`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W69-S01` | done | add canonical finding governance and durable risk acceptance | domain and acceptance checks |
| `W69-S02` | done | expose risk acceptance through API and Postgres | integration checks |
| `W69-S03` | done | let operators accept risk from the findings console | web and e2e checks |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- first governance slice is `risk acceptance`, not suppression
- the write side owns decisions; the read side projects operator-visible state
