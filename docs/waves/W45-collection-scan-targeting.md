# W45. Collection Scan Targeting

Wave: `W45-collection-scan-targeting`
Status: `active`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let operators request one canonical scan batch over a closed release collection so that every managed artifact in the collection becomes a durable scan command target.

## Feature paths

- `features/request-collection-scan.feature`

## Execution lanes

- `unit`
- `integration`
- `infra`
- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/src/inventory/**`
- `crates/venom-domain/src/scanning/**`
- `crates/venom-domain/examples/acceptance.rs`
- `features/request-collection-scan.feature`
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
| `W45-S01` | done | plan canonical scan batches over one closed release collection in the domain | `scripts/check-slice.sh --wave W45-collection-scan-targeting --slice W45-S01 --lane acceptance --path crates/venom-domain/src/inventory --path crates/venom-domain/src/scanning --path features/request-collection-scan.feature` |
| `W45-S02` | in_progress | expose collection scan targeting through the API and durable backends | `scripts/check-slice.sh --wave W45-collection-scan-targeting --slice W45-S02 --lane integration --path apps/api/src/app --path apps/api/src/http --path apps/api/src/infra/postgres_backend.rs` |
| `W45-S03` | planned | let operators trigger collection scans from the UI | `scripts/check-slice.sh --wave W45-collection-scan-targeting --slice W45-S03 --lane e2e --path apps/web/src --path apps/web/e2e` |
| `W45-S04` | planned | close the wave with full docs and gate alignment | `scripts/check-wave.sh --wave W45-collection-scan-targeting` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- a collection scan targets every currently owned immutable artifact of every collection member
- this wave stays operator-triggered; periodic scheduling can follow once collection targeting is stable
