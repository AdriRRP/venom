# W66. UI Visual Language And Process

Wave: `W66-ui-visual-language-and-process`
Status: `done`
BDD impact: `none`
Agentic impact: `compact`
Infra profile: `none`

## Goal

Define one canonical visual language for the VENOM operator console and make it part of the normal delivery process.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `AGENTS.md`
- `docs/ui-visual-language.md`
- `docs/work-methodology.md`
- `docs/repo-structure.md`
- `docs/waves/W66-ui-visual-language-and-process.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W66-S01` | done | define the visual-language wave and its scope | `./scripts/check-slice.sh --wave W66-ui-visual-language-and-process --slice W66-S01 --path docs/waves/W66-ui-visual-language-and-process.md --path docs/waves/ACTIVE` |
| `W66-S02` | done | document the canonical visual language from legacy value plus current state-of-the-art references | `./scripts/check-slice.sh --wave W66-ui-visual-language-and-process --slice W66-S02 --path docs/ui-visual-language.md` |
| `W66-S03` | done | wire the visual language into agent and repository process docs | `./scripts/check-slice.sh --wave W66-ui-visual-language-and-process --slice W66-S03 --path AGENTS.md --path docs/work-methodology.md --path docs/repo-structure.md` |
| `W66-S04` | done | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W66-ui-visual-language-and-process` |

## Language impact

- add `Operator Editorial` as the canonical UI visual direction

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- preserve the legacy product personality without restoring its decorative noise
- keep the console pleasant enough for daily use and strict enough for dense operational work
- treat visual language as product architecture, not optional polish
