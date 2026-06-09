# W240-explicit-legacy-bootstrap-opt-in

Status: `done`

## Goal

Remove the last implicit source-table compatibility from the default Postgres
open contract, so steady-state reopen only trusts canonical repair state and
legacy bootstrap becomes one explicit operator choice.

## Why now

- the remaining repeated findings are no longer about hot-path correctness; they
  are about compatibility still living inside the default open path
- `W239` already isolated legacy source seeding to one bootstrap branch, so the
  next clean step is to make that branch opt-in instead of automatic
- explicit failure is more idiomatic than silently widening the default contract
  when the canonical repair state is missing

## Scope

- require an explicit opt-in to seed provider-report repair state from source
- require an explicit opt-in to seed collection snapshot repair state from
  source
- keep journal/head normalization canonical by default
- document the new explicit bootstrap contract

## Non-goals

- no change to steady-state business behavior
- no new provider-specific flow
- no new `system events` topology work in this wave

## Slices

1. `W240-S01` make legacy source bootstrap opt-in for Postgres open and cover
   both explicit and default-fail reopen paths

## Verification

- targeted `venom-api` Postgres reopen tests for explicit and default legacy
  bootstrap behavior
- `cargo check -p venom-api --all-features --offline`
- `./scripts/check-wave.sh --wave W240-explicit-legacy-bootstrap-opt-in`

## Completion checks

- Glossary impact: none expected
- Invariant impact: default Postgres reopen no longer widens into source
  compatibility implicitly
- BDD impact: none expected
- Reusable workflow impact: none expected
- Documentation compaction opportunity: extend the reliability plan if the
  residual family shrinks again
