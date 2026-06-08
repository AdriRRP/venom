# W236-canonical-repair-journal-closure

Status: `done`

## Goal

Close the remaining cold-repair family by introducing canonical durable repair
journals for provider-report heads and collection snapshots, and tighten the
last meaningful `system events` edge work without opening parallel contracts.

## Why now

- the remaining findings are no longer about hot-path correctness or broad
  architecture; they are about the last repair-only paths that still depend on
  source-wide reconstruction
- previous waves already converged live, detached, and most cold paths around
  canonical state; this wave finishes that story by giving repair paths their
  own durable first-class contract instead of one-off SQL fallbacks
- doing this at the journal level is the most idiomatic way to prevent future
  sibling findings from reappearing under another cold bootstrap wrapper

## Scope

- add one canonical repair journal for latest provider-report heads
- switch provider-report cold repair to journal-first self-healing, with source
  backfill only as compatibility fallback
- add one canonical repair journal for collection snapshot heads
- switch collection cold repair to journal-first self-healing, with source
  backfill only as compatibility fallback
- tighten any remaining `system events` edge/fallback work only where it
  clearly reduces materialization without widening the API contract

## Non-goals

- no new product capability
- no provider-specific behavior
- no semantic BDD change unless truthfulness would otherwise regress

## Slices

1. `W236-S01` add provider-report head repair journal and journal-first cold
   recovery
2. `W236-S02` add collection snapshot repair journal and journal-first cold
   recovery
3. `W236-S03` tighten the last worthwhile `system events` edge allocations

## Verification

- targeted `venom-api` tests for cold rebuild and reopen repair of provider
  report heads and collection snapshots
- targeted `venom-domain` and `venom-api` tests for `system events` edge/fallback
- full `./scripts/check-wave.sh --wave W236-canonical-repair-journal-closure`

## Completion checks

- Glossary impact: none expected
- Invariant impact: repair paths become canonical and durable, not wider
- BDD impact: none expected
- Reusable workflow impact: none expected
- Documentation compaction opportunity: extend the reliability plan only if
  this collapses the residual family cleanly
