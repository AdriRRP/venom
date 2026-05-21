# W91. Release Projection Consolidation

Wave: `W91-release-projection-consolidation`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Move the compact release-board projection into `venom-domain` and make the API
reuse that canonical read model instead of maintaining one parallel
release-scoped shape with duplicated health aggregation logic.

## Feature paths

- `apps/web/src/routes/operations.tsx`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/findings/release_dashboard.rs`
- `crates/venom-domain/src/findings/mod.rs`
- `crates/venom-domain/src/lib.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W91-S01` | done | add one canonical reusable release-board projection in the domain crate | `unit` |
| `W91-S02` | done | consume that projection from the API read snapshot and remove the parallel API-local release board | `unit`, `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
