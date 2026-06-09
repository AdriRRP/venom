# W243-maintenance-build-opt-in-and-event-query-closure

Status: `done`

## Goal

Finish closing the residual runtime-edge family by making legacy repair
bootstrap an explicit maintenance build capability and deleting the last owned
generic `system events` query surface from the domain/runtime contract.

## Why now

- `W242` removed maintenance command parsing from the serve binary, but the
  default runtime build still compiles the legacy source-bootstrap path
- the remaining `SystemEventsPage`/`query()` shape survives only for tests and
  keeps one avoidable generic owned query surface alive in domain code
- closing both together removes another “still technically there” family
  without reopening core hot-path architecture

## Scope

- gate legacy repair bootstrap behind an explicit maintenance build feature
- keep the default `venom-api` runtime build free of legacy source-bootstrap
  execution code
- remove `SystemEventsPage` and generic owned `query()` helpers from
  non-essential domain code
- migrate remaining tests onto cache-native mapped event query helpers

## Non-goals

- no new business behavior
- no redesign of local `system events` merge topology
- no change to the canonical repair-state schema

## Slices

1. `W243-S01` move legacy bootstrap behind maintenance build opt-in and retire
   the last owned generic `system events` query helpers

## Verification

- `cargo check -p venom-api --offline`
- `cargo check -p venom-api --all-features --offline`
- targeted `venom-api` legacy-bootstrap maintenance test with `--all-features`
- targeted `venom-domain` `system events` query tests
- `./scripts/check-wave.sh --wave W243-maintenance-build-opt-in-and-event-query-closure`

## Completion checks

- Glossary impact: none expected
- Invariant impact: default runtime build no longer carries legacy
  source-bootstrap execution code; generic owned `system events` query pages no
  longer exist as a domain contract
- BDD impact: none expected
- Reusable workflow impact: maintenance bootstrap becomes an explicit build
  capability instead of an ambient runtime artifact
- Documentation compaction opportunity: update the reliability plan if the
  residual family narrows again
