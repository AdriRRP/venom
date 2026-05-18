# W70. Suppress findings

Wave: `W70-suppress-findings`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Add one second durable governance decision for active findings so operators can
suppress one finding explicitly while keeping the decision visible and
rebuildable across release-scoped and artifact-scoped views.

## Feature paths

- `features/suppress-finding.feature`

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
- `features/suppress-finding.feature`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W70-S01` | done | extend finding governance with durable suppression and read projection | unit and acceptance checks |
| `W70-S02` | done | expose suppression through API and Postgres | integration checks |
| `W70-S03` | done | let operators suppress findings from the findings console | web and e2e checks |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- suppression stays visible as governance state instead of hiding the finding
- the write side owns decisions; the read side projects operator-visible state
