# W124. Bulk Cohort Stream Compaction

Wave: `W124-bulk-cohort-stream-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Stop materializing full matched bulk-governance cohorts before filtering for
actual state change.

## Feature paths

- `features/accept-risk.feature`
- `features/suppress-finding.feature`
- `features/reopen-finding.feature`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/findings/finding_read_model.rs`
- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W124-S01` | done | compute `targeted` and changed findings in one pass for bulk collection/tag governance actions | `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
