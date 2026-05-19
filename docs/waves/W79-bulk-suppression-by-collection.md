# W79. Bulk Suppression By Collection

Wave: `W79-bulk-suppression-by-collection`
Status: `in_progress`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let operators suppress one filtered open cohort of findings inside one managed
release collection through domain, API, and UI without widening the write model
 or relying on paged fan-out.

## Feature paths

- `features/bulk-suppress-finding.feature`

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
- `features/bulk-suppress-finding.feature`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W79-S01` | done | add one durable batch suppression path over one filtered collection scope | `unit`, `acceptance` |
| `W79-S02` | done | expose the batch action through API and Postgres durability | `integration` |
| `W79-S03` | in_progress | add one operator-facing bulk suppression action to the findings console and cover it end to end | `web`, `e2e` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- Batch governance targets one filtered open cohort only.
- The selected cohort comes from one scoped read projection, not from UI page shape.
