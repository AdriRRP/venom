# W170. Remote Refresh Sub-Lane Narrowing

Wave: `W170-remote-refresh-sub-lane-narrowing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Move detached Postgres refreshes below coarse inventory and read-model lanes so
small remote changes reload only the subgraphs that actually changed.

## Feature paths

- `apps/api/src/infra/postgres_backend.rs`
- `apps/api/src/app/service.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W170-S01` | done | split coarse detached refresh lane masks so collection schedules and provider runtime configs no longer force one full inventory reload | `cargo check -p venom-api --all-features --offline` |
| `W170-S02` | done | reuse unchanged source arcs across detached sub-lane refreshes and keep unrelated-schema probes green | `cargo test -p venom-api postgres_remote_change_probe_ignores_unrelated_schema_writes --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
