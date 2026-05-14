# W05. GitHub Enforcement Activation Attempt

Wave: `W05-github-enforcement-activation-attempt`
Status: `done`
BDD impact: `none`
Agentic impact: `docs`
Infra profile: `none`

## Goal

Attempt live activation of the GitHub required-check enforcement and make the remaining platform constraints explicit and automatable.

## Feature paths

- `none`

## Execution lanes

- `none`

## Owned paths

- `scripts/configure-github-required-checks.sh`
- `docs/runbooks/github-required-checks.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W05-S01` | done | verify remote and GitHub authentication | `git remote -v`, `gh auth status`, `gh repo view AdriRRP/venom --json name,visibility,defaultBranchRef` |
| `W05-S02` | done | attempt live ruleset application and harden the error path | `./scripts/configure-github-required-checks.sh --mode apply` |
| `W05-S03` | done | document the plan and branch-state constraint clearly | inspect updated runbook |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- live application initially failed because rulesets are unavailable for private repositories on the current plan
- after making the repository public, the required-check ruleset was applied successfully
- the remote still has no default branch or pushed `main` head yet
