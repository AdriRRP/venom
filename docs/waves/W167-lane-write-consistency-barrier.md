# W167. Lane Write Consistency Barrier

Wave: `W167-lane-write-consistency-barrier`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep the partitioned `state`, `runtime`, and `publication` HTTP write lanes
truthful by preventing one lane from refreshing stale state and then operating
while a conflicting durable write interleaves in another lane.

## Feature paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W167-S01` | done | introduce cross-lane write coordination that blocks stale state/runtime/publication decisions from interleaving unsafely | `cargo test -p venom-api detached_snapshot_publication_is_monotonic --all-features --offline` |
| `W167-S02` | done | keep Postgres state writes and dependent runtime/publication decisions aligned under one consistency barrier | `cargo test -p venom-api postgres_write_path_refreshes_remote_findings_before_governance_mutation --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
