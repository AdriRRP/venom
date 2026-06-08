# W235-residual-cold-path-and-event-edge-closure

Status: `done`

## Goal

Close the remaining residual findings by removing the last parallel cold-path
rebuilds and tightening `system events` query exposure around cache-native
shapes.

## Why now

- the residual findings are no longer about correctness drift in the hot path;
  they are about leftover bootstrap and edge-query shapes that still do extra
  work
- previous waves already converged live and detached semantics, so the most
  stable next move is to converge cold and operator-facing edge paths too
- this is the best point to prevent new sibling findings from reappearing under
  different rebuild or materialization wrappers

## Scope

- remove the eager schema-time rebuild of `provider_report_heads` and let the
  authoritative snapshot self-heal lazily from canonical durable source data
- collapse the collections cold fallback onto one compact canonical load path
  instead of three source queries plus in-memory assembly
- expose `system events` queries and recent windows through tighter cache-native
  edges so operator-facing response building avoids avoidable intermediate
  materialization
- reduce local merged `system events` fallback work by reusing the same tighter
  query/window contracts

## Non-goals

- no new product capability
- no provider-specific behavior
- no semantic BDD change unless truthfulness would otherwise regress

## Slices

1. `W235-S01` replace the eager provider-report cold backfill with lazy
   authoritative self-healing
2. `W235-S02` collapse the collections cold fallback onto one compact canonical
   load path
3. `W235-S03` tighten `system events` public/query edges around cache-native
   contracts
4. `W235-S04` reuse the tighter `system events` contracts in the local merged
   fallback path

## Verification

- targeted `venom-api` tests for cold rebuild and collection snapshot recovery
- targeted `venom-domain` and `venom-api` tests for `system events` query and
  local merged fallback behavior
- full `./scripts/check-wave.sh --wave W235-residual-cold-path-and-event-edge-closure`

## Completion checks

- Glossary impact: none expected
- Invariant impact: cold and edge paths become more canonical, not wider
- BDD impact: none expected
- Reusable workflow impact: none expected
- Documentation compaction opportunity: update the reliability plan only if
  this closes the residual family cleanly
