# W111. HTTP Mutation Lane Parallelism

Wave: `W111-http-mutation-lane-parallelism`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce time spent inside the single-flight HTTP mutation lane by making volatile
snapshot refreshes cheaper.

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `crates/venom-domain/src/durable_state.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W111-S01` | done | shrink mutation-lane refresh cost through cheap volatile-lane cache reuse | `unit`, `integration` |

## Invariant impact

`I8`, `I11`
