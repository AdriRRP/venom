# W38. UI Ecosystem Selection

Wave: `W38-ui-ecosystem-selection`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Select the most suitable UI ecosystem for a first VENOM operator console, using the current architecture and performance/reliability targets as the decision frame.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `docs/ui-ecosystem-evaluation.md`
- `docs/waves/W38-ui-ecosystem-selection.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W38-S01` | done | define the UI ecosystem selection wave and target | `./scripts/check-slice.sh --wave W38-ui-ecosystem-selection --slice W38-S01 --path docs/waves/ACTIVE --path docs/waves/W38-ui-ecosystem-selection.md` |
| `W38-S02` | done | document the ecosystem evaluation, persist the active-wave pointer, and recommend one default path for VENOM | `./scripts/check-slice.sh --wave W38-ui-ecosystem-selection --slice W38-S02 --path docs/waves/ACTIVE --path docs/ui-ecosystem-evaluation.md` |
| `W38-S03` | planned | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W38-ui-ecosystem-selection` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- treat the first UI as an operator console, not a marketing website
- prefer stacks that do not duplicate the Rust backend already in place
- optimize for maintainability, deterministic delivery, and lean runtime behavior
