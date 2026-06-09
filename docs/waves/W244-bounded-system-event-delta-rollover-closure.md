# W244-bounded-system-event-delta-rollover-closure

Status: `done`

## Goal

Close the last recurring local observability residual by making bounded
`system events` delta detection robust across recent-window rollover and using
that stronger delta contract before any local merged-window rebuild fallback.

## Why now

- the post-`W243` residual is no longer about veracity or runtime-edge drift;
  it is one bounded local merge fallback under churn
- the current delta helper only recognizes exact suffix reuse, so bounded
  append churn that evicts old tail entries still falls back to full merge
- fixing delta semantics at the domain level is more idiomatic than adding
  more local cache topology

## Scope

- extend bounded recent-window delta detection to support append-with-eviction
- let the local merged `system events` snapshot reuse dual-side deltas when
  both state and runtime advanced within the bounded cache horizon
- add directed regression coverage for rollover and dual-side bounded churn

## Non-goals

- no change to business behavior
- no new observability storage topology
- no redesign of Postgres-backed `system events`

## Slices

1. `W244-S01` extend bounded delta semantics across rollover and tighten local
   dual-side merged snapshot refresh

## Verification

- targeted `venom-domain` system-event delta rollover tests
- targeted `venom-api` local merged system-event cache tests
- `cargo check -p venom-api --all-features --offline`
- `./scripts/check-wave.sh --wave W244-bounded-system-event-delta-rollover-closure`

## Completion checks

- Glossary impact: none expected
- Invariant impact: bounded local `system events` deltas remain truthful even
  after recent-window tail eviction
- BDD impact: none expected
- Reusable workflow impact: none expected
- Documentation compaction opportunity: update the reliability plan if the last
  local observability residual disappears
