# W182. Read Model Inner Source Sharing

Wave: `W182-read-model-inner-source-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove avoidable deep clones from `FindingReadModel` by moving its active and
decision maps onto inner copy-on-write arcs.

## Feature paths

- `crates/venom-domain/src/findings/finding_read_model.rs`

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/findings/finding_read_model.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W182-S01` | done | share `FindingReadModel` sources across cloned lanes and only copy inner maps on mutation through `Arc::make_mut` | `cargo test -p venom-domain finding_read_model --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
