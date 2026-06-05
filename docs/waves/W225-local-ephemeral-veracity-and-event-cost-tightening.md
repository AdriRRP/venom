# W225. Local Ephemeral Veracity And Event Cost Tightening

Wave: `W225-local-ephemeral-veracity-and-event-cost-tightening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Close the local ephemeral lane correctness regressions introduced by `W224`
and tighten the remaining hot-path cost around merged local `system events`
without regressing truthful operator-facing behavior.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W225-S01` | done | make local ephemeral lane reopen failures explicit instead of silently retrying forever | `cargo test -p venom-api local_ephemeral_runtime_lane_reopen_failure_is_explicit --all-features --offline` |
| `W225-S02` | done | converge local ephemeral lane reopen to a truthful local epoch before reusing the lane | `cargo test -p venom-api local_ephemeral_runtime_lane_reopen_converges_after_concurrent_state_change --all-features --offline` |
| `W225-S03` | done | avoid residual retained-ref rebuild churn inside `SystemEventQueryIndex` merge and delta paths | `cargo test -p venom-domain system_event_query_index_merged_reuses_incremental_retained_refs --all-features --offline` |
| `W225-S04` | done | tighten the local merged `system events` fallback so it reuses cached windows more aggressively before rebuilding | `cargo test -p venom-api local_merged_system_event_snapshot_reuses_cached_windows_after_state_reopen --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I2`, `I3`, `I8`, `I11`

## ADR impact

`none`
