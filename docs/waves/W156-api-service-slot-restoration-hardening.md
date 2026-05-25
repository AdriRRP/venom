# W156. API Service Slot Restoration Hardening

Wave: `W156-api-service-slot-restoration-hardening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Always restore the live `ApiApplication` slot before returning errors from
refresh or watermark-observation paths, and publish the freshest visible
snapshot after successful writes or remote refreshes.

## Feature paths

- `none`

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W156-S01` | done | restore the live service slot on all refresh and watermark-observation exits, and publish refreshed snapshots even when a write path returns an explicit infrastructure error | `./scripts/check-wave.sh --wave W156-api-service-slot-restoration-hardening` |

## Language impact

`none`

## Invariant impact

`I2, I8, I9, I11`

## ADR impact

`none`
