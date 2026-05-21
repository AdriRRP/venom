# W88. Veracity and Read-Model Hardening

Wave: `W88-veracity-and-read-model-hardening`
Status: `done`
BDD impact: `refine`
Agentic impact: `script`
Infra profile: `db`

## Goal

Close three reliability and efficiency gaps that currently make VENOM look more complete than it is: acceptance and browser gates must not pass on skipped coverage, contextual views must expose truthful effective-context provenance, and release-scoped operator views must avoid redundant full-scope passes.

## Feature paths

- `features/view-bulk-governance-workbench.feature`
- `features/view-collection-governance.feature`
- `features/manage-context-profiles.feature`
- `features/classify-finding.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `web`
- `e2e`

## Owned paths

- `scripts/check-acceptance.sh`
- `scripts/check-web-e2e.sh`
- `crates/venom-domain/examples/acceptance.rs`
- `crates/venom-domain/src/findings/**`
- `crates/venom-domain/src/inventory/**`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/findings.tsx`
- `apps/web/src/routes/operations.tsx`
- `features/**`
- `docs/architecture-invariants.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W88-S01` | done | fail gates when acceptance or browser smoke skip observable coverage | `acceptance`, `e2e` |
| `W88-S02` | done | project truthful effective-context provenance instead of a misleading single profile identity | `unit`, `integration`, `acceptance`, `web` |
| `W88-S03` | done | reduce redundant full-scope passes in release-scoped operator views | `unit`, `integration` |

## Language impact

`change`

## Invariant impact

`I5`, `I8`, `I9`, `I11`

## ADR impact

`none`

## Notes

- Browser smoke may still skip under sandbox restrictions, but that must not count as a passing verification lane inside the governed wave path.
