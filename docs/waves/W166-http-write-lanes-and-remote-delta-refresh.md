# W166. HTTP Write Lanes And Remote Delta Refresh

Wave: `W166-http-write-lanes-and-remote-delta-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Close the remaining structural `P2` block by removing the single-flight
bottleneck on the serious Postgres write path, narrowing remote refresh from
full detached rebuilds to changed read-side lanes, making the `ReleaseBoard`
lazy over source arcs, and compacting retained system-event indexing.

## Feature paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `crates/venom-domain/src/findings/finding_ingestion.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`
- `crates/venom-domain/src/durable_state.rs`

## Execution lanes

- `unit`
- `integration`
- `web`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `crates/venom-domain/src/findings/finding_ingestion.rs`
- `crates/venom-domain/src/operations/system_event_trace.rs`
- `crates/venom-domain/src/durable_state.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W166-S01` | done | make `ReleaseBoard` lazy over `inventory + read_model` source arcs instead of rebuilding eagerly on every refresh | `cargo check -p venom-api --all-features --offline` |
| `W166-S02` | done | compact `SystemEventQueryIndex` so retained events are stored once and referenced cheaply across bounded windows | `cargo check -p venom-domain --all-features --offline` |
| `W166-S03` | done | narrow Postgres remote refresh to changed lanes using a change journal instead of detached full rebuilds | `cargo test -p venom-api detached_postgres_fresh_read_promotes_the_observed_change_watermark --all-features --offline` |
| `W166-S04` | done | partition the Postgres HTTP write plane into `state`, `runtime`, and `publication` service lanes | `cargo check -p venom-api --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`

## Notes

This wave changes runtime topology and hot-path refresh behavior, not the
observable product semantics of findings, collections, or governance.
