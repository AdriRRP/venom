# W241-legacy-bootstrap-maintenance-separation

Status: `done`

## Goal

Remove legacy source bootstrap from the normal Postgres service-open path
entirely, and expose it only as one explicit maintenance operation.

## Why now

- `W240` already made legacy bootstrap opt-in, which shrank the family of
  repeated findings substantially
- the clean final step is to stop carrying compatibility switches inside the
  service-open contract at all
- that leaves steady-state startup fully canonical and moves upgrade-only
  compatibility into one operator-chosen maintenance action

## Scope

- remove legacy bootstrap toggles from normal API/Postgres open paths
- add one explicit maintenance entrypoint for legacy repair bootstrap:
  `venom-api bootstrap-legacy-repair-state`
- keep tests for both fail-fast canonical open and successful explicit legacy
  bootstrap
- update reliability planning for the narrower residual contract

## Non-goals

- no business behavior change
- no new provider-specific path
- no new `system events` redesign in this wave

## Slices

1. `W241-S01` separate legacy repair bootstrap from service startup and cover
   the explicit maintenance flow

## Verification

- targeted `venom-api` Postgres repair-bootstrap tests
- `cargo check -p venom-api --all-features --offline`
- `./scripts/check-wave.sh --wave W241-legacy-bootstrap-maintenance-separation`

## Completion checks

- Glossary impact: none expected
- Invariant impact: service startup is canonical-only; legacy bootstrap is
  maintenance-only
- BDD impact: none expected
- Reusable workflow impact: if the maintenance flow stabilizes, keep it as one
  explicit command rather than another runtime switch
- Documentation compaction opportunity: extend the reliability plan only if the
  residual family shrinks again
