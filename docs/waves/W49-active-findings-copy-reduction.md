# W49. Active Findings Copy Reduction

Wave: `W49-active-findings-copy-reduction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reduce resident memory and copy churn in the active findings read path by storing one narrower operator-facing projection instead of full provider-facing finding objects, while preserving current observable behavior.

## Feature paths

- `features/view-active-findings.feature`

## Execution lanes

- `unit`
- `integration`
- `infra`
- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/src/findings/**`
- `crates/venom-domain/benches/hot_paths.rs`
- `docs/waves/W49-active-findings-copy-reduction.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W49-S01` | done | compact active findings storage to a narrower operator projection and keep active query behavior stable | `scripts/check-slice.sh --wave W49-active-findings-copy-reduction --slice W49-S01 --lane unit --path crates/venom-domain/src/findings --path crates/venom-domain/benches/hot_paths.rs` |
| `W49-S02` | done | close the wave with docs and full gate alignment | `scripts/check-wave.sh --wave W49-active-findings-copy-reduction` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- prefer explicit compact value objects over clever lifetimes or unsafe tricks
- do not sacrifice deterministic replay or readable operator query code
- optimize only hot paths that are already measured or directly implicated by measured paths
