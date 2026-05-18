# W67. Agentic Context Compaction

Wave: `W67-agentic-context-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `compact`
Infra profile: `none`

## Goal

Reduce duplicated agent guidance and leave the repository with the smallest reliable agentic surface that still preserves delivery discipline.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `AGENTS.md`
- `CONTRIBUTING.md`
- `docs/documentation-model.md`
- `docs/waves/W67-agentic-context-compaction.md`
- `agents/skills/venom-delivery/SKILL.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W67-S01` | done | define the wave and target | `./scripts/check-slice.sh --wave W67-agentic-context-compaction --slice W67-S01 --path docs/waves/W67-agentic-context-compaction.md --path docs/waves/ACTIVE` |
| `W67-S02` | done | remove redundant agentic explanation and keep one compact canonical model | `./scripts/check-slice.sh --wave W67-agentic-context-compaction --slice W67-S02 --path docs/documentation-model.md --path AGENTS.md` |
| `W67-S03` | done | compress remaining optional entrypoints and skill guidance | `./scripts/check-slice.sh --wave W67-agentic-context-compaction --slice W67-S03 --path CONTRIBUTING.md --path agents/skills/venom-delivery/SKILL.md` |
| `W67-S04` | done | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W67-agentic-context-compaction` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- the agentic system should optimize for one short manifest, one compact documentation model, and script-first reuse
- optional skills may exist, but they must add orchestration value rather than restate the manifest
