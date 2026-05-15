# W22. Dependency Surface Hardening

Wave: `W22-dependency-surface-hardening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove unused SQLx backend surface so the default dependency graph matches the Postgres-only runtime and the required security audit passes cleanly.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/Cargo.toml`
- `Cargo.lock`
- `docs/waves/W22-dependency-surface-hardening.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W22-S01` | done | disable unused SQLx default backends and simplify Postgres row mapping so audit tracks a smaller real runtime surface | `cargo test --workspace --all-targets --all-features`, `./scripts/check-audit.sh` |
| `W22-S02` | done | reset the active wave pointer after closing the security hardening wave | `./scripts/check-wave.sh --wave W22-dependency-surface-hardening` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- the default runtime path remains Postgres-only
- reducing dependency surface is part of reliability and security hardening
- `audit` ignores `RUSTSEC-2023-0071` only because `sqlx` keeps mysql-only optional dependencies in the lockfile even though VENOM does not ship that backend
