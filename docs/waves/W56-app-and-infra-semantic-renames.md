# W56. App and Infra Semantic Renames

Wave: `W56-app-and-infra-semantic-renames`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Rename the main app and infrastructure types so their names describe their role directly: API application orchestration, Postgres-backed store, and HTTP event publishing.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/http_integration_publisher.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/waves/W56-app-and-infra-semantic-renames.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W56-S01` | done | rename app and infra types to role-facing names without changing behavior | `scripts/check-slice.sh --wave W56-app-and-infra-semantic-renames --slice W56-S01 --lane integration --path apps/api/src/app/service.rs --path apps/api/src/http/mod.rs --path apps/api/src/infra/http_integration_publisher.rs --path apps/api/src/infra/postgres_backend.rs` |
| `W56-S02` | done | close the wave with docs and full gate alignment | `scripts/check-wave.sh --wave W56-app-and-infra-semantic-renames` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- prefer role-facing names like `ApiApplication` and `PostgresStore` over generic `service` and `backend`
- final integrated validation is performed again after the full W54-W57 naming sequence lands cleanly
