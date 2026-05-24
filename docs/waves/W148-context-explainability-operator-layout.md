# W148. Context Explainability Operator Layout

Wave: `W148-context-explainability-operator-layout`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Make contextual explanation read like operator information instead of debug
text by separating summary, posture, rule, and factor evidence.

## Execution lanes

- `web`

## Owned paths

- `apps/web/src/routes/findings.tsx`
- `apps/web/src/styles.css`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W148-S01` | done | render contextual semantics as structured operator content with lightweight visual hierarchy | `web` |

## Language impact

`none`

## Invariant impact

`I9`

## ADR impact

`none`
