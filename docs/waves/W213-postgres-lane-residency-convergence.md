# W213. Postgres Lane Residency Convergence

Wave: `W213-postgres-lane-residency-convergence`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Converge idle Postgres lane residency back onto the latest restored lane so
forked services do not keep long-lived duplicate hot arcs after successful
mutations.

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
| `W213-S01` | done | rebase idle sibling Postgres lanes onto the latest restored lane residency after successful mutations | `cargo test -p venom-api postgres_idle_lanes_rebase_to_latest_resident_sources --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
