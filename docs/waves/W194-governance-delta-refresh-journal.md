# W194. Governance Delta Refresh Journal

Wave: `W194-governance-delta-refresh-journal`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Make Postgres findings refresh consume durable governance deltas instead of
reloading full acceptance and suppression tables whenever the governance lane
changes.

## Feature paths

- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W194-S01` | done | persist governance changes into one append-only journal and replay/rebuild from it | `cargo test -p venom-api postgres_reopened_findings_are_replayed_from_governance_journal --all-features --offline` |
| `W194-S02` | done | delta-refresh read-model governance state from governance journal cursors instead of reloading full tables | `cargo test -p venom-api detached_postgres_read_snapshot_advances_governance_journal_cursor_for_reopened_findings --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
