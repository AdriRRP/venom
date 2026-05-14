# W21. Provider Runtime Config

Wave: `W21-provider-runtime-config`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Resolve the execution provider from durable managed-component configuration instead of passing it ad hoc in worker payloads.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `apps/api/**`
- `crates/venom-domain/**`
- `docs/ubiquitous-language.md`
- `docs/waves/W21-provider-runtime-config.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W21-S01` | done | add durable provider runtime configuration for managed components and make worker execution resolve provider choice from that state | `./scripts/check-quality.sh`, `cargo test --workspace --all-targets --all-features`, `./scripts/rehearse-infra.sh --profile db`, `./scripts/check-wave.sh --wave W21-provider-runtime-config` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- worker execution must fail explicitly when a pending command has no configured provider runtime
- provider choice belongs to managed runtime state, not to one-off worker payloads
