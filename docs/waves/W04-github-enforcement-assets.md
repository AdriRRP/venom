# W04. GitHub Enforcement Assets

Wave: `W04-github-enforcement-assets`
Status: `done`
BDD impact: `none`
Agentic impact: `script`
Infra profile: `none`

## Goal

Remove the last manual ambiguity around required checks by materializing importable and scriptable GitHub enforcement assets, even before a remote repository is configured.

## Feature paths

- `none`

## Execution lanes

- `none`

## Owned paths

- `infra/github/**`
- `scripts/configure-github-required-checks.sh`
- `docs/runbooks/github-required-checks.md`
- `docs/repo-structure.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W04-S01` | done | add an importable ruleset payload for `main` | inspect `infra/github/main-required-checks.ruleset.json` |
| `W04-S02` | done | add a deterministic apply script for GitHub rulesets | `bash -n scripts/configure-github-required-checks.sh`, `./scripts/configure-github-required-checks.sh --owner example --repo venom --mode dry-run` |
| `W04-S03` | done | document the current local limitation and the canonical application path | inspect updated runbook |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- the local workspace still cannot apply the ruleset automatically because no GitHub remote is configured here
