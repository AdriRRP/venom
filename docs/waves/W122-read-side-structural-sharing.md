# W122. Read Side Structural Sharing

Wave: `W122-read-side-structural-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Share prebuilt release-board and read-side lanes across HTTP snapshots instead
of rebuilding those projections inside `ApiReadSnapshot`.

## Feature paths

- `apps/api/src/app/service.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W122-S01` | done | reuse cached `ReleaseBoard` and lane-specific shared snapshots across local and Postgres-backed reads | `unit`, `integration` |

## Language impact

- none

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
