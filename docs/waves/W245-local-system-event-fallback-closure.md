# W245-local-system-event-fallback-closure

Status: `done`

## Goal

Close the last residual local `system events` merge fallback by making bounded
delta application the canonical refresh path for cached merged snapshots.

## Scope

- simplify local merged `system events` cache refresh around the bounded-delta
  contract introduced in `W244`
- keep one explicit cold/exceptional rebuild path only for cases that violate
  the append-only lane contract
- add regression coverage for dual-side bounded rollover so this family does
  not reopen through a nearby edge

## Out of scope

- generic public query-surface micro-optimizations
- any new persistence or Postgres changes

## Slices

1. Replace the residual local merged-snapshot fallback tree with one bounded
   delta refresh path plus one cold/exceptional rebuild path.
2. Add regression coverage for dual-lane recent-window rollover.

## Verification

- `cargo test -p venom-api local_bounded_lane_updates_cover_dual_side_recent_window_rollover --all-features --offline`
- `cargo test -p venom-api local_merged_system_event_snapshot_reuses_cached_windows_across_bounded_single_side_churn --all-features --offline`
- `cargo check -p venom-api --all-features --offline`
- `./scripts/check-wave.sh --wave W245-local-system-event-fallback-closure`

## Documentation checks

- glossary impact: none
- invariant impact: none
- BDD impact: none
- reusable workflow impact: none
- documentation compaction opportunity: this wave closes the last residual
  branch of the local `system events` family instead of introducing new side
  guidance
