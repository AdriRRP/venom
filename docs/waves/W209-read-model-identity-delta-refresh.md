# W209. Read Model Identity Delta Refresh

Wave: `W209-read-model-identity-delta-refresh`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Narrow Postgres findings and governance refreshes to the changed identities so
the read-model lane stops replaying whole watermark ranges when only a subset
of artifacts or findings changed.

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
| `W209-S01` | done | load latest provider reports and governance rows only for identities touched since the last cursor | `cargo test -p venom-api postgres_read_model_delta_refresh_reloads_only_changed_identities --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
