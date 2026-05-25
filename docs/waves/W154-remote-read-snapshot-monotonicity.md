# W154. Remote Read Snapshot Monotonicity

Wave: `W154-remote-read-snapshot-monotonicity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Publish detached Postgres-backed fresh-read snapshots only when their change
watermark is newer than the snapshot already visible to operators.

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
| `W154-S01` | done | guard detached snapshot publication by monotonic remote watermarks and cover the pure decision rules | `cargo test -p venom-api http::tests --all-features` |

## Language impact

`none`

## Invariant impact

`I8, I9, I11`

## ADR impact

`none`
