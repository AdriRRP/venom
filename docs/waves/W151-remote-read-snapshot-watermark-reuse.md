# W151. Remote Read Snapshot Watermark Reuse

Wave: `W151-remote-read-snapshot-watermark-reuse`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reuse one detached remote read snapshot until the schema-local watermark changes
again instead of rebuilding on every fresh read while the main write store
stays stale.

## Feature paths

- `none`

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W151-S01` | done | cache detached remote snapshots by schema-local watermark | `cargo test -p venom-api postgres_backend --all-features` |

## Language impact

`none`

## Invariant impact

`I8, I11`

## ADR impact

`none`

