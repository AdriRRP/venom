# W187. State Runtime Barrier Scope Tightening

Wave: `W187-state-runtime-barrier-scope-tightening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Keep the state/runtime consistency barrier only around runtime mutations that
actually need inventory-read correctness at decision time.

## Feature paths

- `apps/api/src/http/mod.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W187-S01` | done | remove the state read barrier concept from runtime HTTP lanes now that durable revalidation is the correctness mechanism, while preserving state-write exclusion | `cargo test -p venom-api runtime_and_publication_lanes_do_not_take_the_state_consistency_barrier --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
