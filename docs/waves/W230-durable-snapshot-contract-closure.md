# W230-durable-snapshot-contract-closure

Status: `done`

## Goal

Close the remaining recurring structural findings by unifying detached/live
durable cursors, moving collection snapshot refresh toward collection-key
targeted reloads, and tightening the last operator-facing `system events`
materialization path.

## Why now

- recent waves already removed the broad rebuild patterns
- the remaining findings now cluster around a small set of repeated contracts
- closing those contracts directly is the best way to stop circular follow-up
  findings on the same families

## Scope

- Postgres detached read-model cursor contract
- Postgres collection snapshot refresh granularity
- operator-facing `system events` query edge cost

## Non-goals

- no new product capability
- no BDD shape change unless observable behavior changes
- no provider-specific semantics

## Slices

1. `W230-S01` unify detached/live durable cursors, collection-key targeted
   collection refresh, and cache-native `system events` edge materialization

## Verification

- targeted `venom-domain` tests for `system events` query/page behavior
- targeted `venom-api` tests for detached Postgres snapshot refresh
- full `./scripts/check-wave.sh --wave W230-durable-snapshot-contract-closure`

## Completion checks

- Glossary impact: none
- Invariant impact: none
- BDD impact: none
- Reusable workflow impact: none
- Documentation compaction opportunity: folded back into the reliability plan
