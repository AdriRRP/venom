# W163. Contextual Audit Surface

Wave: `W163-contextual-audit-surface`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Present contextual risk semantics as an operator-facing audit surface instead of
mixed chips and prose fragments.

## Feature paths

- `apps/web/src/routes/findings.tsx`

## Execution lanes

- `web`

## Owned paths

- `apps/web/src/routes/findings.tsx`
- `apps/web/src/routes/findings.test.tsx`
- `apps/web/src/styles.css`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W163-S01` | done | render contextual posture, rule, and factor provenance as structured audit rows and factor tables | `npm --prefix apps/web run check` |

## Language impact

`none`

## Invariant impact

`I11`

## ADR impact

`none`

## Notes

This wave does not change contextual semantics. It changes only the operator
surface so auditability matches the richer provenance already present in the
API.
