# W144. Collection Governance Overview Bounded Page

Wave: `W144-collection-governance-overview-bounded-page`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep release-scoped governance pages bounded to the requested offset window
instead of materializing every matched finding before sorting and slicing.

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/findings/collection_governance_overview.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W144-S01` | done | retain full health and bulk-governance totals while keeping only the requested top-k page window in memory | `unit` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
