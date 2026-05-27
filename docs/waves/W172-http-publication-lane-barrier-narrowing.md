# W172. HTTP Publication Lane Barrier Narrowing

Wave: `W172-http-publication-lane-barrier-narrowing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Stop forcing publication-only HTTP mutations through the same cross-lane
consistency barrier used for state-dependent runtime decisions, while keeping
durable correctness intact.

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
| `W172-S01` | done | remove unnecessary publication participation from the state/runtime consistency barrier | `cargo test -p venom-api --all-features --offline -- --nocapture` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
