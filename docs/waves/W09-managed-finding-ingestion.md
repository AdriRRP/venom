# W09. Managed Finding Ingestion

Wave: `W09-managed-finding-ingestion`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `none`

## Goal

Require provider scan reports to reference components that are already under management before VENOM ingests them.

## Feature paths

- `features/report-finding.feature`

## Execution lanes

- `unit`
- `acceptance`

## Owned paths

- `docs/waves/W09-managed-finding-ingestion.md`
- `features/report-finding.feature`
- `crates/venom-domain/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W09-S01` | done | add a minimal ingestion service that rejects unmanaged components and extend canonical finding-ingestion acceptance coverage | `./scripts/check-acceptance.sh`, `cargo test --workspace --all-targets --all-features`, `./scripts/check-wave.sh --wave W09-managed-finding-ingestion` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep the rule at the domain boundary, not hidden inside acceptance-only glue
