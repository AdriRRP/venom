# W13. Durable State And Read Model

Wave: `W13-durable-state-and-read-model`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `none`

## Goal

Add a minimal durable history and a rebuildable read model so managed ownership and active findings survive reloads without inventing hidden infrastructure.

## Feature paths

- `features/view-active-findings.feature`

## Execution lanes

- `unit`
- `acceptance`

## Owned paths

- `docs/product-direction.md`
- `docs/ubiquitous-language.md`
- `docs/waves/W13-durable-state-and-read-model.md`
- `features/view-active-findings.feature`
- `crates/venom-domain/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W13-S01` | done | add an append-only durable history and a rebuildable active findings projection over managed ownership | `cargo test --workspace --all-targets --all-features`, `./scripts/check-acceptance.sh`, `./scripts/check-wave.sh --wave W13-durable-state-and-read-model` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- durable history is append-only JSON lines for now
- active findings are rebuilt from durable provider snapshots rather than stored as a separate source of truth
