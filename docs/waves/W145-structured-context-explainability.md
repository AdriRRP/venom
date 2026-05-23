# W145. Structured Context Explainability

Wave: `W145-structured-context-explainability`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Render contextual profile, posture, rule, and effective factors as structured
operator-facing UI content so context decisions read like product behavior
rather than a debug string.

## Execution lanes

- `web`

## Owned paths

- `apps/web/src/routes/findings.tsx`
- `apps/web/src/routes/findings.test.tsx`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W145-S01` | done | replace the dense context label with structured, readable operator content while preserving the same truthful data | `web` |

## Language impact

`none`

## Invariant impact

`I9`

## ADR impact

`none`
