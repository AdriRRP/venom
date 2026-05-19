# W80. Bulk Governance Workbench

Wave: `W80-bulk-governance-workbench`
Status: `in_progress`
BDD impact: `create`
Agentic impact: `docs`
Infra profile: `none`

## Goal

Turn collection-scoped bulk governance into one explicit operator workbench
with one dedicated read-side cohort summary and one consistent action flow over
the currently filtered open cohort.

## Feature paths

- `features/view-bulk-governance-workbench.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `web`
- `e2e`

## Owned paths

- `crates/venom-domain/src/findings/**`
- `crates/venom-domain/examples/acceptance.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/lib/api.test.ts`
- `apps/web/src/routes/findings.tsx`
- `apps/web/src/routes/findings.test.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `features/view-bulk-governance-workbench.feature`
- `docs/debt-closure-plan.md`
- `docs/product-direction.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W80-S01` | done | define the debt-closure sequence and add one dedicated bulk-governance cohort projection to the domain | `unit`, `acceptance` |
| `W80-S02` | planned | expose the bulk-governance workbench summary through the collection findings API | `integration` |
| `W80-S03` | planned | replace the duplicated bulk forms with one operator workbench flow in the findings console | `web`, `e2e` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
