# W78. Bulk Risk Acceptance By Collection

Wave: `W78-bulk-risk-acceptance-by-collection`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let operators accept risk for one filtered open cohort of findings inside one
managed release collection through domain, API, and UI without widening the
write model or relying on paged fan-out.

## Feature paths

- `features/bulk-accept-risk.feature`

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
- `features/bulk-accept-risk.feature`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W78-S01` | done | add one durable batch risk-acceptance path over one filtered collection scope | `unit`, `acceptance` |
| `W78-S02` | done | expose the batch action through API and Postgres durability | `integration` |
| `W78-S03` | done | add one operator-facing bulk action to the findings console and cover it end to end | `web`, `e2e` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- Batch governance targets one filtered open cohort only.
- The selected cohort comes from one scoped read projection, not from UI page shape.
