# W242-runtime-edge-contract-closure

Status: `completed`

## Goal

Finish separating residual maintenance and observability edge concerns from the
runtime contract so the steady-state service path stays as small and idiomatic
as possible.

## Why now

- `W241` already removed legacy bootstrap from the normal Postgres open path,
  but the maintenance entrypoint still lives in the main service binary
- the generic owned `SystemEventsPage` contract is no longer needed in
  production paths because the API already uses mapped cache-native queries
- closing both edges together removes another family of “still technically
  there” residuals without reopening hot-path architecture

## Scope

- move legacy repair bootstrap into a dedicated maintenance binary
- restore the main API binary to a pure serve path
- retire the generic public `SystemEventsPage` query surface from non-test
  runtime code
- keep internal/system-event test coverage on the retained test-only query path

## Non-goals

- no business behavior change
- no new provider-specific path
- no new redesign of local system-event merge topology

## Slices

1. `W242-S01` isolate maintenance bootstrap from the serve binary and tighten
   the generic system-event query contract to test-only usage

## Verification

- targeted `venom-api` tests for separated legacy bootstrap behavior
- `cargo check -p venom-api --all-features --offline`
- `./scripts/check-wave.sh --wave W242-runtime-edge-contract-closure`

## Completion checks

- Glossary impact: none expected
- Invariant impact: runtime serve path no longer carries maintenance command
  parsing; generic owned system-event page is no longer a production contract
- BDD impact: none expected
- Reusable workflow impact: maintenance stays explicit and separate instead of
  leaking back into normal runtime controls
- Documentation compaction opportunity: update the reliability plan if the
  residual family narrows again
