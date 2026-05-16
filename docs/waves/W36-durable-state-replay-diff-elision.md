# W36. Durable State Replay Diff Elision

Wave: `W36-durable-state-replay-diff-elision`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce durable state replay cost by restoring tracker state without computing `FindingChangeSet` diffs that are not consumed during replay.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/findings/finding_ingestion.rs`
- `crates/venom-domain/src/findings/finding_tracker.rs`
- `crates/venom-domain/src/durable_state.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W36-S01` | in_progress | define the durable state replay diff elision wave and target | `./scripts/check-slice.sh --wave W36-durable-state-replay-diff-elision --slice W36-S01 --path docs/waves/ACTIVE --path docs/waves/W36-durable-state-replay-diff-elision.md` |
| `W36-S02` | planned | add replay-only ingestion/tracker paths that skip unused diff computation during durable rebuild | `cargo test --workspace --all-targets --all-features && ./scripts/check-performance-baseline.sh` |
| `W36-S03` | planned | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W36-durable-state-replay-diff-elision` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep replay semantics identical
- optimize only the rebuild path, not the observable write-time change set contract
