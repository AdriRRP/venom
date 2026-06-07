# W232-authoritative-snapshot-edge-closure

Status: `completed`

## Goal

Close the remaining recurring structural findings by introducing authoritative
compact snapshot heads for cold provider-report rebuilds, one canonical joined
collection snapshot loader, and one final tightening pass on the operator-facing
`system events` edge.

## Why now

- the remaining findings no longer point to correctness gaps, only to cold-path
  and edge-shape duplication
- previous waves already converged live and detached delta semantics, so this is
  the right moment to converge the remaining rebuild contracts too
- closing these contracts directly is the best way to avoid another circular
  round of findings on adjacent rebuild paths

## Scope

- Postgres authoritative latest-provider-report heads for cold rebuild
- Postgres authoritative joined collection snapshot loading
- residual operator-facing `system events` edge tightening where it can be done
  without widening hot-path cost

## Non-goals

- no new product capability
- no provider-specific semantics
- no BDD behavior change unless a truthfulness defect appears

## Slices

1. `W232-S01` add authoritative provider-report heads and move cold rebuild to
   them plus the finding journal
2. `W232-S02` replace multi-query collection snapshot assembly with one joined
   canonical collection snapshot loader
3. `W232-S03` remove any remaining avoidable production use of public
   `system events` window materialization

## Verification

- targeted `venom-api` tests for cold rebuild and collection snapshot refresh
- targeted `venom-domain` and `venom-api` checks for `system events`
- full `./scripts/check-wave.sh --wave W232-authoritative-snapshot-edge-closure`

## Completion checks

- Glossary impact: none expected
- Invariant impact: none expected
- BDD impact: none expected
- Reusable workflow impact: none expected
- Documentation compaction opportunity: fold the family conclusion back into the
  reliability plan if the residual list clearly shrinks again
