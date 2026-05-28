# W195. Live Observability Tail Refresh

Wave: `W195-live-observability-tail-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Replace live Postgres lane reloads for command statuses, pending integration
events, and system events with cursor-driven tail refreshes.

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
| `W195-S01` | done | tail-refresh live command statuses and pending integration events from durable row cursors | `cargo test -p venom-api postgres_live_refresh_reloads_pending_commands_incrementally --all-features --offline` |
| `W195-S02` | done | tail-refresh live system events from durable event cursors instead of lane reloads | `cargo test -p venom-api postgres_live_refresh_reloads_system_events_incrementally --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
