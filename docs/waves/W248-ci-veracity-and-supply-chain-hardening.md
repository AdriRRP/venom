# W248-ci-veracity-and-supply-chain-hardening

Status: `done`
Infra profile: `db`

## Goal

Make required CI truthful and reproducible by exercising the real PostgreSQL
integration path, auditing the committed Rust and npm dependency graphs, and
keeping advisory workflows out of the required pull-request path.

## Scope

- run PostgreSQL-backed tests in the required `tests` workflow
- fail required CI when PostgreSQL integration coverage is not configured
- audit the committed Cargo and npm lockfiles without regenerating them
- remediate current high-severity frontend dependency advisories
- use deterministic dependency installation and keep advisory triggers aligned
- prevent local runtime state from entering the repository accidentally

## Non-goals

- defining a deployment target or release promotion policy
- adding review-count requirements to the GitHub ruleset
- changing domain or API behavior

## Slices

1. `W248-S01` make required tests exercise PostgreSQL-backed coverage
2. `W248-S02` audit exact Rust and npm locks and remediate current advisories
3. `W248-S03` align deterministic installs, advisory triggers, and repo hygiene

## Verification

- `VENOM_REQUIRE_POSTGRES_TESTS=1 ./scripts/check-tests.sh` fails without a database URL
- `./scripts/check-audit.sh`
- `./scripts/check-wave.sh --wave W248-ci-veracity-and-supply-chain-hardening`

## Completion checks

- Glossary impact: none
- Invariant impact: none
- BDD impact: none
- Reusable workflow impact: CI reuses repository-owned gates
- Documentation compaction opportunity: keep the required-check contract in
  `docs/work-methodology.md` and the GitHub ruleset runbook

## Notes

- the PostgreSQL rehearsal exercises 58 persistence and integration tests that
  previously returned early in required CI
- the first required PostgreSQL run exposed a deterministic legacy-repair bug:
  collection snapshots were rebuilt before their component and context-profile
  prerequisites; the repair lane now loads those prerequisites first
- exact-lock auditing exposed and remediated two Rust vulnerabilities and four
  high-severity npm advisories that the previous audit path did not report
