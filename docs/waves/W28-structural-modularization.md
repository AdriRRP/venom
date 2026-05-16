# W28. Structural Modularization

Wave: `W28-structural-modularization`
Status: `active`
BDD impact: `none`
Agentic impact: `compact`
Infra profile: `none`

## Goal

Reduce structural entropy in the domain and API code before adding more capabilities, while keeping behavior unchanged and preserving the current performance and reliability posture.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/**`
- `apps/api/src/**`
- `docs/repo-structure.md`
- `docs/adr/0004-internal-module-organization.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W28-S01` | done | define the internal module organization target for domain and API code | `./scripts/check-slice.sh --wave W28-structural-modularization --slice W28-S01 --path docs/waves/ACTIVE --path docs/waves/W28-structural-modularization.md --path docs/repo-structure.md --path docs/adr/0004-internal-module-organization.md` |
| `W28-S02` | done | regroup domain modules by capability without changing public semantics | `cargo test --workspace --all-targets --all-features` |
| `W28-S03` | in_progress | regroup API modules by boundary and remove the flat router/service layout | `cargo test --workspace --all-targets --all-features` |
| `W28-S04` | planned | compact docs, close the wave, and run the full wave gate | `./scripts/check-wave.sh --wave W28-structural-modularization` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`0004-internal-module-organization`

## Notes

- do not change observable behavior in this wave
- prefer mechanical moves and visibility tightening over new abstractions
