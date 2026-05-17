# W41. UI Scan Command Loop

Wave: `W41-ui-scan-command-loop`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Extend the operator console so one operator can observe one scan command status and execute one fixture-backed worker drain from the UI for manual end-to-end validation.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/web/**`
- `docs/waves/ACTIVE`
- `docs/waves/W41-ui-scan-command-loop.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W41-S01` | planned | define the wave and the manual scan loop boundary | `./scripts/check-slice.sh --wave W41-ui-scan-command-loop --slice W41-S01 --path docs/waves/ACTIVE --path docs/waves/W41-ui-scan-command-loop.md` |
| `W41-S02` | planned | add canonical web API operations for scan command status and worker drain | `./scripts/check-slice.sh --wave W41-ui-scan-command-loop --slice W41-S02 --lane unit --path apps/web/src` |
| `W41-S03` | planned | add operator UI for command status lookup and fixture worker execution | `./scripts/check-slice.sh --wave W41-ui-scan-command-loop --slice W41-S03 --lane unit --path apps/web/src` |
| `W41-S04` | planned | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W41-ui-scan-command-loop` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep worker execution explicit and visibly fixture-backed
- keep the manual loop useful for real browser validation without inventing hidden backend behavior
