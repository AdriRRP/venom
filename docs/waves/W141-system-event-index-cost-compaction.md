# W141. System Event Index Cost Compaction

Wave: `W141-system-event-index-cost-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce avoidable intermediate allocations in bulk-governance cohort handling by
streaming matched findings directly into the durable write shape each caller
actually needs.

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
| `W141-S01` | done | add one visitor-based bulk-governance matching path and use it in local and Postgres bulk mutations to skip intermediate finding vectors and conversions | `unit`, `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
