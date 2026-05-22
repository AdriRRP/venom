# W117. Local Read-Side Deep Clone Elision

Wave: `W117-local-read-side-deep-clone-elision`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Cache deep local read-side lanes as `Arc` snapshots so inventory and finding
read-model refreshes stop cloning entire structures on demand.

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W117-S01` | done | add cached `Arc<ComponentInventory>` and `Arc<FindingReadModel>` lanes to local durable state and Postgres-backed state, and consume them from the API snapshot builder | `unit`, `integration` |

## Invariant impact

`I8`, `I11`
