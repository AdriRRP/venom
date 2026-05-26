# W165. Read Model Source Arc Sharing

Wave: `W165-read-model-source-arc-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Eliminate deep `FindingReadModel` clones on read-side snapshot refresh by
keeping the live projection under shared `Arc` ownership and using copy-on-
write only when one mutation actually changes it.

## Feature paths

- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W165-S01` | done | keep the live `FindingReadModel` under `Arc` ownership and refresh snapshot lanes by cloning the source `Arc` instead of cloning the whole projection | `cargo test -p venom-domain read_model_snapshot_cache_reuses_live_read_model_arc --all-features` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`

## Notes

This wave changes storage topology, not observable finding semantics.
