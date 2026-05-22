# W107. Bulk Governance State Compaction

Wave: `W107-bulk-governance-state-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove avoidable full-structure clones from bulk governance actions by deriving
the changed cohort from current state, appending the durable event, and then
applying only the changed findings back into governance and read-model state.

## Feature paths

- `features/accept-finding-risk.feature`
- `features/suppress-finding.feature`
- `features/reopen-finding.feature`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W107-S01` | done | stop cloning whole governance and read-model structures before bulk accept, suppress, and reopen actions | `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
