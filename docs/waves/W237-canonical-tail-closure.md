# W237 Canonical Tail Closure

Status: `done`

## Why

The remaining findings are no longer about functional truthfulness, but about
residual compatibility branches and bounded but still avoidable event-query
cost. The right closure is not more ad hoc fixes in hot paths; it is one
canonical bootstrap contract for durable repair state plus one tighter
system-event edge contract.

## Scope

- move provider-report and collection repair compatibility into canonical
  bootstrap normalization
- keep rebuild and detached loaders on authoritative/journal state instead of
  source-wide fallback branches
- tighten the local merged `system events` fallback when recent windows are
  unchanged but totals drift
- remove avoidable intermediate `Vec` materialization from the generic
  `SystemEventQueryIndex::query()` edge

## Non-goals

- redesign operator-facing `system events` semantics
- introduce new business behavior
- widen BDD scope

## Slices

### W237-S01 canonical bootstrap and event-tail closure

Status: `done`

Goals:

- normalize authoritative repair tables during Postgres open
- simplify runtime loaders so compatibility fallback no longer lives in the
  steady-state path
- reuse cached merged recent windows when only totals changed outside the
  visible window
- keep the generic `system events` edge borrowed/cache-native until the final
  owned response boundary

## Verification

- targeted `venom-api` Postgres reopen tests for provider-report heads and
  collection snapshots
- targeted `venom-api` local merged `system events` tests
- targeted `venom-domain` tests for generic `SystemEventQueryIndex::query()`
- `cargo check -p venom-api --all-features --offline`
- `./scripts/check-wave.sh --wave W237-canonical-tail-closure`

## Impact check

- Glossary impact: none expected
- Invariant impact: strengthens canonical repair-state ownership
- BDD impact: none expected
- Reusable workflow impact: none expected
- Documentation compaction opportunity: keep residual-closure guidance in this
  wave instead of duplicating it elsewhere
