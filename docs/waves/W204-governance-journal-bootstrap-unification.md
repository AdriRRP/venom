# W204. Governance Journal Bootstrap Unification

Wave: `W204-governance-journal-bootstrap-unification`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Make the governance journal the canonical Postgres bootstrap source for
governance state and read-model replay so cold rebuilds no longer depend on
legacy governance tables.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W204-S01` | done | rebuild governance state from journal snapshots instead of legacy acceptance and suppression tables | `cargo test -p venom-api postgres_rebuild_restores_governance_from_journal_snapshot --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I5`, `I8`, `I11`

## ADR impact

`none`
