# W155. Fresh Read Watermark Fast Path

Wave: `W155-fresh-read-watermark-fast-path`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reuse one already-published detached Postgres read snapshot without taking the
remote refresh lane when the schema-local watermark has not advanced.

## Feature paths

- `none`

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W155-S01` | done | short-circuit fresh Postgres-backed reads from the published watermark before they serialize on the remote refresh lane | `cargo test -p venom-api postgres_backend --all-features` |

## Language impact

`none`

## Invariant impact

`I8, I11`

## ADR impact

`none`
