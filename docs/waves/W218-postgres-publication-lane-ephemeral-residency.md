# W218. Postgres Publication Lane Ephemeral Residency

Wave: `W218-postgres-publication-lane-ephemeral-residency`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Remove the fixed always-live third Postgres service from `ApiState` steady
state by turning the publication lane into an ephemeral fork that only exists
while publication work is actually running.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W218-S01` | done | make Postgres publication services lazy/ephemeral while preserving publication correctness and lane refresh semantics | `cargo test -p venom-api postgres_publication_lane_is_ephemeral_until_taken --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
