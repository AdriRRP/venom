# W203. Postgres Live State Fork Compaction

Wave: `W203-postgres-live-state-fork-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reduce the residual resident-state cost of keeping two live Postgres-backed API
lanes by sharing more of the forked live state surface instead of duplicating
lane-local mutable structures eagerly.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W203-S01` | done | compact forked Postgres live-state residency across state and volatile API lanes | `cargo test -p venom-api postgres_live_lanes_share_forked_state_residency --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
