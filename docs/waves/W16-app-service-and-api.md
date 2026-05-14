# W16. App Service And Api

Wave: `W16-app-service-and-api`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Expose the current durable inventory and finding flow through a minimal HTTP API and an explicit application service, without changing domain semantics.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/**`
- `crates/venom-domain/src/finding_read_model.rs`
- `docs/waves/W16-app-service-and-api.md`
- `.github/workflows/dependency-freshness.yml`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W16-S01` | done | move dependency freshness back to a truly advisory workflow so PR delivery is not blocked by a non-required noisy gate | workflow inspection, `gh pr checks 1` |
| `W16-S02` | in_progress | add an application service and minimal HTTP API for component registration, artifact binding, provider report ingestion, and active finding queries | `cargo test --workspace --all-targets --all-features`, `./scripts/check-wave.sh --wave W16-app-service-and-api` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- the API uses local durable state and keeps the domain provider-agnostic
- API transport coverage lives in Rust integration tests rather than canonical business `.feature` files
