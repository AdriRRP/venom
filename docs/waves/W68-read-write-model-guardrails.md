# W68. Read Write Model Guardrails

Wave: `W68-read-write-model-guardrails`
Status: `done`
BDD impact: `none`
Agentic impact: `docs`
Infra profile: `none`

## Goal

Make the separation between durable write paths and efficient rebuildable read paths an explicit architectural guardrail, so the current system cannot drift back toward the legacy coupling mistakes.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `docs/architecture-invariants.md`
- `docs/ubiquitous-language.md`
- `docs/work-methodology.md`
- `docs/waves/W68-read-write-model-guardrails.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W68-S01` | done | define the guardrail wave and target | `./scripts/check-slice.sh --wave W68-read-write-model-guardrails --slice W68-S01 --path docs/waves/W68-read-write-model-guardrails.md --path docs/waves/ACTIVE` |
| `W68-S02` | done | persist the architectural and language guardrails for write/read separation and efficient projections | `./scripts/check-slice.sh --wave W68-read-write-model-guardrails --slice W68-S02 --path docs/architecture-invariants.md --path docs/ubiquitous-language.md --path docs/work-methodology.md` |
| `W68-S03` | done | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W68-read-write-model-guardrails` |

## Language impact

- add `Write Model`
- add `Read Model`

## Invariant impact

- add one explicit invariant for write/read separation and projection-specific reads

## ADR impact

`none`

## Notes

- current code already follows this shape through durable write paths, rebuildable domain projections, and API read snapshots
- the purpose of this wave is to make that shape non-optional for future changes
