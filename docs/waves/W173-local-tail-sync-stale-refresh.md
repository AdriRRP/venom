# W173. Local Tail Sync Stale Refresh

Wave: `W173-local-tail-sync-stale-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep partitioned local HTTP lanes truthful without paying full reopen-and-replay
from disk whenever one lane falls behind another.

## Feature paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W173-S01` | done | track replay offsets in local durable histories and tail-apply only appended events | `cargo test -p venom-domain --all-features --offline tail_sync -- --nocapture` |
| `W173-S02` | done | switch local stale-lane refresh from reopen to incremental tail sync with fallback safety | `cargo test -p venom-api --all-features --offline -- --nocapture` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
