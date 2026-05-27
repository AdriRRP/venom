# W192. State Runtime Barrier Elision

Wave: `W192-state-runtime-barrier-elision`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove the leftover state/runtime consistency barrier if lane ownership already
provides the necessary serialization boundary.

## Feature paths

- `apps/api/src/http/mod.rs`

## Execution lanes

- `unit`

## Owned paths

- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W192-S01` | done | drop redundant state/runtime barrier locking and keep lane invariants explicit in tests | `cargo test -p venom-api runtime_and_publication_lanes_do_not_take_the_state_consistency_barrier --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
