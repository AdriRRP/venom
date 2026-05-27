# W184. Postgres Lane Residency Sharing

Wave: `W184-postgres-lane-residency-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Stop duplicating the hot Postgres scan-command and integration-outbox
residency across forked `ApiState` lanes when one bootstrapped lane is reused
for the others.

## Feature paths

- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W184-S01` | done | share command runtime and integration outbox residency across forked Postgres lanes through copy-on-write `Arc` sources instead of cloning them at bootstrap | `cargo test -p venom-api postgres_fork_shares_runtime_and_outbox_sources_until_lane_mutation --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
