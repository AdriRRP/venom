# W231-durable-journal-granularity-closure

Status: `done`

## Goal

Close the remaining recurring structural findings by making the durable delta
contracts authoritative in cold rebuilds, collection refresh, and the last
operator-facing `system events` edge.

## Why now

- recent waves already removed the broad rebuild and clone-heavy patterns
- the remaining findings now cluster around a small set of durable-contract
  mismatches rather than independent bugs
- closing those contracts directly is the best way to avoid another circular
  family of follow-up findings on the same paths

## Scope

- Postgres cold read-model rebuild granularity
- Postgres collection refresh journal compactness
- residual `system events` edge shape and merge fallback tightening

## Non-goals

- no new product capability
- no provider-specific semantics
- no change to the observable BDD contract unless a truthfulness bug is found

## Slices

1. `W231-S01` move cold read-model rebuild onto the finding-level durable
   journal and remove the last wide provider-report replay path
2. `W231-S02` introduce one compact collection change journal and refresh
   collections from changed collection keys and sections instead of table-local
   fan-out
3. `W231-S03` tighten the remaining `system events` cache-native rebuild edge
   so Postgres snapshots stop bouncing through public recent-window shapes

## Verification

- targeted `venom-api` tests for cold rebuild and collection delta refresh
- targeted `venom-domain` tests for `system events` cache/query shape
- full `./scripts/check-wave.sh --wave W231-durable-journal-granularity-closure`

## Completion checks

- Glossary impact: none expected
- Invariant impact: none expected
- BDD impact: none expected
- Reusable workflow impact: none expected
- Documentation compaction opportunity: fold the final conclusion back into the
  reliability plan if the family closes cleanly
