# W100. Bulk Cohort Streaming

Wave: `W100-bulk-cohort-streaming`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove avoidable ordering and projection work from bulk-governance cohorts so
mass actions operate on lean `FindingRef` streams instead of full
operator-facing finding views.

## Feature paths

- none

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
| `W100-S01` | done | switch bulk-governance actions to lean finding-ref cohorts without stable ordering or unused projection payloads | `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
