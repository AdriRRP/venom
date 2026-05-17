# W44. Release Collections

Wave: `W44-release-collections`
Status: `active`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let operators define one closed release collection over managed components, expose it through the API, and operate it from the UI as the canonical scope for later periodic scanning.

## Feature paths

- `features/manage-collections.feature`

## Execution lanes

- `unit`
- `integration`
- `infra`
- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/src/inventory/**`
- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/examples/acceptance.rs`
- `features/manage-collections.feature`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/operations.tsx`
- `apps/web/src/routes/operations.test.tsx`
- `apps/web/e2e/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W44-S01` | done | define closed release collections in the domain and durable state | `scripts/check-slice.sh --wave W44-release-collections --slice W44-S01 --lane acceptance --path crates/venom-domain/src/inventory crates/venom-domain/src/durable_state.rs features/manage-collections.feature` |
| `W44-S02` | done | expose release collection commands and queries through the API and Postgres backend | `scripts/check-slice.sh --wave W44-release-collections --slice W44-S02 --lane integration --path apps/api/src/app apps/api/src/http apps/api/src/infra/postgres_backend.rs` |
| `W44-S03` | in_progress | let operators manage release collections from the UI | `scripts/check-slice.sh --wave W44-release-collections --slice W44-S03 --lane e2e --path apps/web/src apps/web/e2e` |
| `W44-S04` | planned | close the wave with full docs and gate alignment | `scripts/check-wave.sh --wave W44-release-collections` |

## Language impact

`change`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- collections are closed explicit scopes over managed components only
- collection membership is intentionally component-based in this wave; artifact-level collection targeting can follow once release scopes are established
