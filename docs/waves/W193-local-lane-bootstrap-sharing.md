# W193. Local Lane Bootstrap Sharing

Wave: `W193-local-lane-bootstrap-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce local `ApiState` bootstrap and residency cost by opening one local
durable application view and forking the second lane from that in-memory base
instead of reopening both histories from disk.

## Feature paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`

## Execution lanes

- `unit`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W193-S01` | done | fork the local volatile lane from one opened application base and keep snapshot sharing explicit in tests | `cargo test -p venom-api local_open_forks_volatile_lane_from_one_bootstrap_base --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
