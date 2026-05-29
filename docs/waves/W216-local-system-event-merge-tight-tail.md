# W216. Local System Event Merge Tight Tail

Wave: `W216-local-system-event-merge-tight-tail`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep local merged system-event windows on the append-delta fast path for more
cases before falling back to a full bounded recomposition.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W216-S01` | done | reuse cached merged windows from longer append tails instead of bounded full merge fallback | `cargo test -p venom-api local_merged_system_event_snapshot_reuses_cached_peer_window_for_longer_append_tails --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`

## ADR impact

`none`
