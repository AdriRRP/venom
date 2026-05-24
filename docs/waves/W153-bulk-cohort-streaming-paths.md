# W153. Bulk Cohort Streaming Paths

Wave: `W153-bulk-cohort-streaming-paths`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Make visitor-based streaming paths the canonical bulk-governance API and remove
leftover vector-materializing helpers from the read model surface.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/findings/finding_read_model.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W153-S01` | done | remove vector-collecting bulk helpers and assert full cohort coverage through streaming visitors | `cargo test -p venom-domain finding_read_model --all-features` |

## Language impact

`none`

## Invariant impact

`I8, I11`

## ADR impact

`none`

