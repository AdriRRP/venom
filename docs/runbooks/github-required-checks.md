# GitHub Required Checks

## Purpose

Make the repository-enforced checks real in GitHub, not only defined in workflow files.

## Configure

Preferred path:

- apply `infra/github/main-required-checks.ruleset.json`
- or run `scripts/configure-github-required-checks.sh --mode apply`

The ruleset must target `main` and require these checks:

- `quality`
- `tests`
- `audit`

Recommended:

- require branches to be up to date before merge through the status-check policy
- add pull-request review rules separately if the repository workflow needs them
- include merge queue support if the repository uses it

Do not require at this stage:

- `unused-deps`

Reason:

- it is useful, but still better treated as advisory until it proves stable across real delivery waves

## Review rule

Recheck this list when:

- a new gate becomes part of `scripts/check-wave.sh`
- a heavy gate becomes stable enough for normal PR flow
- a gate becomes noisy enough that it should move out of the required path

## Access requirement

Applying from this checkout requires one of:

- `gh auth login`
- `GITHUB_TOKEN`
- `GH_TOKEN`

GitHub platform constraints still apply when repository visibility or plan changes:

- repository rulesets are available for private repositories only on GitHub Pro, Team, or Enterprise
- protected branches are also gated by plan for private repositories
- branch protection also needs a real branch to protect

Verify after apply:

1. the repository has an active ruleset targeting `refs/heads/main`
2. the required contexts are `quality`, `tests`, and `audit`
3. first branch creation is allowed, but later updates to `main` are governed by the required checks
4. add separate pull-request review rules later only if the workflow needs them
