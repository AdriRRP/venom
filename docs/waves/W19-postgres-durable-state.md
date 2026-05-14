# W19. Postgres Durable State

Wave: `W19-postgres-durable-state`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Add a real Postgres-backed durable app backend that preserves the current inventory, finding, and scan-command contracts while keeping the local file-backed path available.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `apps/api/**`
- `crates/venom-domain/**`
- `infra/**`
- `scripts/infra-smoke.sh`
- `docs/waves/W19-postgres-durable-state.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W19-S01` | done | add a Postgres-backed durable app backend, preserve the current HTTP contracts, and prove reload durability locally | `./scripts/check-quality.sh`, `cargo test --workspace --all-targets --all-features`, `./scripts/check-slice.sh --wave W19-postgres-durable-state --slice W19-S01 --lane integration` |
| `W19-S02` | done | wire the db infra rehearsal against Docker Compose and validate the Postgres backend against real local infrastructure | `./scripts/rehearse-infra.sh --profile db` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- Postgres must remain a backend detail behind the app boundary
- the local JSONL path remains the default local-first fallback
- `postgres:18-alpine` is the first Postgres 18 alpine tag that worked correctly on this ARM64 host
