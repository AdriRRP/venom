# W73. Deterministic contextual risk

Wave: `W73-deterministic-contextual-risk`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let operators see one deterministic contextual risk level for active findings
based on raw severity plus one managed component context profile, without
moving that derived meaning into the write model.

## Feature paths

- `features/classify-finding.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/src/findings/**`
- `crates/venom-domain/src/inventory/**`
- `crates/venom-domain/examples/acceptance.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/lib/api.test.ts`
- `apps/web/src/routes/findings.tsx`
- `apps/web/src/routes/findings.test.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `features/classify-finding.feature`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W73-S01` | done | add deterministic contextual risk derivation in the domain and acceptance flow | unit and acceptance checks |
| `W73-S02` | done | expose contextual risk through API read projections and Postgres-backed reload paths | integration checks |
| `W73-S03` | done | show contextual risk and component context in the findings console and browser smoke | web and e2e checks |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
