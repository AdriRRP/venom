# W10. Managed Artifact Ownership

Wave: `W10-managed-artifact-ownership`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `none`

## Goal

Require a managed component to explicitly own an immutable artifact before VENOM accepts provider scan reports for that artifact.

## Feature paths

- `features/register-component.feature`
- `features/report-finding.feature`

## Execution lanes

- `unit`
- `acceptance`

## Owned paths

- `docs/ubiquitous-language.md`
- `docs/waves/W10-managed-artifact-ownership.md`
- `features/register-component.feature`
- `features/report-finding.feature`
- `crates/venom-domain/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W10-S01` | done | add explicit artifact ownership to inventory and reject finding reports for artifacts the managed component does not own | `./scripts/check-acceptance.sh`, `cargo test --workspace --all-targets --all-features`, `./scripts/check-wave.sh --wave W10-managed-artifact-ownership` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- ownership must be explicit and deterministic
- one immutable artifact must not silently belong to two components
