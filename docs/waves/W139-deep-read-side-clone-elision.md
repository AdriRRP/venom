# W139. Deep Read-Side Clone Elision

Wave: `W139-deep-read-side-clone-elision`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Stop rebuilding whole command-status maps for ordinary enqueue and status
transitions when one incremental snapshot update is enough.

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W139-S01` | done | update command-status snapshot lanes incrementally in local and Postgres hot paths instead of rebuilding the full map on each transition | `unit`, `integration` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`
