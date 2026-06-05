# W224. Local Volatile Lane Ephemeral Residency

Wave: `W224-local-volatile-lane-ephemeral-residency`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove the fixed resident local `runtime` and `publication` lanes from
steady-state `ApiState` by reopening those volatile lanes from durable local
disk only when they are actually taken, while preserving truthful local
mutation visibility across lane boundaries.

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
| `W224-S01` | done | make the local runtime lane ephemeral-until-taken with durable reopen semantics from disk | `cargo test -p venom-api local_runtime_lane_is_ephemeral_until_taken --all-features --offline` |
| `W224-S02` | done | make the local publication lane ephemeral-until-taken without reintroducing runtime/publication slot coupling | `cargo test -p venom-api local_publication_lane_is_ephemeral_until_taken --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
