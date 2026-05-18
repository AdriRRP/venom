# W75. Collection governance workbench

Wave: `W75-collection-governance-workbench`
Status: `in_progress`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let operators work one release collection from the findings screen with one
scoped findings page plus one compact governance and risk summary derived from
the same read-side projection.

## Feature paths

- `features/view-collection-governance.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
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
- `features/view-collection-governance.feature`
- `docs/product-direction.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W75-S01` | in_progress | compose one collection governance overview from release-scoped findings and collection health | unit and acceptance checks |
| `W75-S02` | pending | expose collection governance overview through the collection findings API | integration checks |
| `W75-S03` | pending | show the collection governance workbench on the findings screen with quick filters | web and e2e checks |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`
