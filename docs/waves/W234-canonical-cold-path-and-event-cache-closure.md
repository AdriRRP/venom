# W234-canonical-cold-path-and-event-cache-closure

Status: `done`

## Goal

Close the remaining residual structural findings by removing parallel cold-path
rebuild logic and consolidating `system events` retained/cache state around one
authoritative shape.

## Why now

- the remaining findings are all variants of the same problem: residual edge
  paths still rebuild or rehydrate from secondary shapes instead of the
  canonical live contracts
- previous waves already converged live and detached paths for the hot system
  behavior, so the best next move is to make cold/self-healing paths converge
  too
- this is the highest-leverage point to stop circular findings from reappearing
  under slightly different rebuild or query wrappers

## Scope

- replace eager SQL authoritative-table backfill with canonical self-healing
  rebuilds for active findings and collections
- make cold Postgres replay recover authoritative snapshot tables from the same
  contracts already used by the live model
- move retained `system events` ref-state into the recent-window cache shape so
  cache rehydration does not rebuild parallel state
- tighten `system events` query and local fallback paths around the unified
  cache shape

## Non-goals

- no new product capability
- no provider-specific behavior
- no observable BDD change unless truthfulness would otherwise regress

## Slices

1. `W234-S01` remove eager SQL backfill and make findings/collections cold
   rebuild self-heal from canonical source contracts
2. `W234-S02` unify retained `system events` ref-state with the cache shape and
   remove parallel rehydration
3. `W234-S03` tighten local fallback and query edges to consume the unified
   cache shape directly

## Verification

- targeted `venom-api` tests for reopen, cold rebuild, and detached refresh of
  findings and collections
- targeted `venom-domain` and `venom-api` tests for `system events`
- full `./scripts/check-wave.sh --wave W234-canonical-cold-path-and-event-cache-closure`

## Completion checks

- Glossary impact: none expected
- Invariant impact: rebuild paths become more canonical, not semantically wider
- BDD impact: none expected
- Reusable workflow impact: none expected
- Documentation compaction opportunity: fold the remaining residual-family
  closure into the reliability plan if the findings collapse again
