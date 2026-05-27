# W191. Postgres Observability Tail Refresh

Wave: `W191-postgres-observability-tail-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Turn detached Postgres refreshes for `system events` and `command statuses`
into bounded tail syncs instead of full lane reloads.

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
| `W191-S01` | done | tail-refresh Postgres command-status snapshots from durable row cursors instead of reloading the full table | `cargo test -p venom-api postgres_collection_scan_request_reloads_pending_commands --all-features --offline` |
| `W191-S02` | done | tail-refresh Postgres system-event snapshots from durable event cursors and merge them over the visible base snapshot | `cargo test -p venom-api postgres_due_collection_scan_drain_reloads_system_events --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
