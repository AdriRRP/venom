# W246-ci-node24-action-hygiene

Status: `done`

## Goal

Remove the remaining Node 20 deprecation warning from required CI by updating
the pinned `Swatinem/rust-cache` action to a current Node 24-compatible
release.

## Scope

- update required and advisory workflows that pin `Swatinem/rust-cache`
- keep immutable SHA pinning in place
- avoid any unrelated workflow churn

## Non-goals

- changing required-check policy
- wider workflow refactors

## Slices

1. `W246-S01` bump `Swatinem/rust-cache` pins to `v2.9.1`

## Verification

- `./scripts/check-wave.sh --wave W246-ci-node24-action-hygiene`

## Completion checks

- Glossary impact: none
- Invariant impact: none
- BDD impact: none
- Reusable workflow impact: none
- Documentation compaction opportunity: none
