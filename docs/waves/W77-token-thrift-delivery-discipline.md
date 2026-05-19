# W77. Token-Thrift Delivery Discipline

Wave: `W77-token-thrift-delivery-discipline`
Status: `done`
BDD impact: `none`
Agentic impact: `compact`
Infra profile: `none`

## Goal

Persist a low-token communication discipline that keeps delivery strict while
reducing repeated status narration.

## Feature paths

- none

## Execution lanes

- `unit`

## Owned paths

- `AGENTS.md`
- `docs/work-methodology.md`
- `docs/documentation-model.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W77-S01` | done | compact always-on and process docs around delta-only reporting | `unit` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- The repo stays the canonical memory.
- The chat should report deltas, not replay stable status.
