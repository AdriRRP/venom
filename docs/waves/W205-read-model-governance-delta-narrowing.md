# W205. Read Model Governance Delta Narrowing

Wave: `W205-read-model-governance-delta-narrowing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Narrow Postgres read-model refresh cost by replaying only the latest effective
governance delta per finding since the current watermark instead of every
journal row in that interval.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W205-S01` | done | compact governance delta refresh to one effective row per finding identity after the cursor | `cargo test -p venom-api postgres_governance_delta_refresh_replays_only_latest_effective_rows --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
