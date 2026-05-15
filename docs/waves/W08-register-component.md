# W08. Register Component

Wave: `W08-register-component`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `none`

## Goal

Add the first inventory capability so VENOM can explicitly know which components are under management.

## Feature paths

- `features/register-component.feature`

## Execution lanes

- `unit`
- `acceptance`

## Owned paths

- `docs/waves/W08-register-component.md`
- `features/register-component.feature`
- `crates/venom-domain/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W08-S01` | done | add a minimal component inventory with idempotent registration and canonical acceptance coverage | `./scripts/check-acceptance.sh`, `cargo test --workspace --all-targets --all-features`, `./scripts/check-wave.sh --wave W08-register-component` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep the first inventory capability intentionally small and deterministic
