# W233-authoritative-derived-state-closure

Status: `done`

## Goal

Close the last recurring residual findings by making derived cold-path state
authoritative, upgrade-safe, and shared across live and detached loaders.

## Why now

- the remaining findings are no longer about business correctness, but about
  residual rebuild width, upgrade safety, and repeated edge-shape work
- previous waves already converged live and detached delta contracts, so the
  right next step is to converge the remaining derived-state rebuild contracts
- this is the best point to eliminate circular findings around cold rebuilds,
  collection snapshots, and local `system events` fallback recomposition

## Scope

- authoritative active-finding snapshot state for cold Postgres rebuilds
- authoritative collection snapshot heads for cold and targeted reloads
- upgrade-safe backfill of derived authoritative tables on open
- local `system events` fallback merge tightening around already-loaded caches

## Non-goals

- no new product capability
- no provider-specific semantics
- no BDD behavior change unless a truthfulness defect appears

## Slices

1. `W233-S01` add authoritative active-finding snapshot state and use it for
   cold rebuilds
2. `W233-S02` add authoritative collection snapshot heads and use them for cold
   and targeted collection reloads
3. `W233-S03` backfill derived authoritative tables during open so upgrades do
   not depend on prior wave history
4. `W233-S04` tighten local `system events` fallback merge paths to reuse
   already-fetched caches instead of recomputing equivalent windows twice

## Verification

- targeted `venom-api` tests for authoritative backfill and collection/findings
  rebuilds
- targeted `venom-domain` and `venom-api` tests for `system events`
- full `./scripts/check-wave.sh --wave W233-authoritative-derived-state-closure`

## Completion checks

- Glossary impact: none expected
- Invariant impact: derived-state rebuilds become more explicit, no invariant
  change expected
- BDD impact: none expected
- Reusable workflow impact: none expected
- Documentation compaction opportunity: fold the family closure back into the
  reliability plan if the residual list shrinks again
