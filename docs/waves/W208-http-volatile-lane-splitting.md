# W208. HTTP Volatile Lane Splitting

Wave: `W208-http-volatile-lane-splitting`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Split `runtime` and `publication` onto independent volatile HTTP service lanes
so long-running runtime drains stop serializing publication work by sharing one
mutable slot.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W208-S01` | done | split runtime and publication onto separate live API slots with truthful stale refresh semantics | `cargo test -p venom-api runtime_and_publication_lanes_do_not_share_one_service_slot --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
