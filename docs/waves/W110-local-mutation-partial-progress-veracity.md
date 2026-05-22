# W110. Local Mutation Partial Progress Veracity

Wave: `W110-local-mutation-partial-progress-veracity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep HTTP read snapshots truthful even when one local mutation returns an error
after partial durable progress already happened.

## Owned paths

- `apps/api/src/http/mod.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W110-S01` | done | refresh read snapshots after mutation attempts, not only after success | `unit`, `integration` |

## Invariant impact

`I2`, `I3`, `I11`
