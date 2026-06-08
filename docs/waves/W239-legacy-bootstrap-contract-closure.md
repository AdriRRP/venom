# W239-legacy-bootstrap-contract-closure

Status: `done`

## Goal

Move the last source-table compatibility fallback out of canonical repair
loaders and into one explicit legacy bootstrap seeding path, so steady-state
repair and reopen logic stay aligned on one durable contract.

## Why now

- the remaining residual findings are no longer about correctness in the hot
  path; they are about compatibility branches that still live too close to the
  normal reopen flow
- previous waves already converged live, detached, and cold repair around
  authoritative heads and repair journals
- the clean closure is to make legacy seeding explicit and one-time, not to
  keep source-wide fallback embedded in canonical loader logic

## Scope

- introduce explicit legacy bootstrap seeding for provider-report heads
- introduce explicit legacy bootstrap seeding for collection snapshot heads
- keep canonical normalization and live loaders journal/head-only
- update residual documentation to reflect the new contract boundary

## Non-goals

- no business behavior change
- no new provider-specific path
- no further `system events` redesign in this wave

## Slices

1. `W239-S01` move provider-report and collection source fallback into explicit
   legacy bootstrap seeding during open

## Verification

- targeted `venom-api` reopen tests for provider-report and collection repair
  state
- `cargo check -p venom-api --all-features --offline`
- `./scripts/check-wave.sh --wave W239-legacy-bootstrap-contract-closure`

## Completion checks

- Glossary impact: none expected
- Invariant impact: canonical repair loaders no longer own source compatibility
- BDD impact: none expected
- Reusable workflow impact: none expected
- Documentation compaction opportunity: extend the reliability plan only if the
  residual family meaningfully shrinks
