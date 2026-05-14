# W03. Control Plane Hardening

Wave: `W03-control-plane-hardening`
Status: `done`
BDD impact: `none`
Agentic impact: `compact`
Infra profile: `none`

## Goal

Close the remaining control-plane gaps before product implementation: executable wave gates, tighter workflow hardening, leaner always-on guidance, and explicit GitHub enforcement instructions.

## Feature paths

- `none`

## Execution lanes

- `none`

## Owned paths

- `.github/workflows/**`
- `scripts/**`
- `AGENTS.md`
- `CONTRIBUTING.md`
- `agents/skills/venom-delivery/SKILL.md`
- `docs/work-methodology.md`
- `docs/runbooks/github-required-checks.md`
- `docs/waves/W01-foundation.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W03-S01` | done | add executable slice, wave, and heavy gates | `bash -n scripts/*.sh`, `./scripts/check-slice.sh --wave W03-control-plane-hardening --slice W03-S01`, `./scripts/check-wave.sh --wave W03-control-plane-hardening` |
| `W03-S02` | done | pin workflows to immutable action SHAs and document GitHub enforcement | inspect workflow YAML and runbook |
| `W03-S03` | done | compact the always-on agent and contributor guidance | inspect updated docs and skill |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- the wave gates now fail explicitly if acceptance, e2e, or contract artifacts appear before their runners are wired
