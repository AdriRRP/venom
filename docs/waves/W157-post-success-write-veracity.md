# W157. Post-Success Write Veracity

Wave: `W157-post-success-write-veracity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Do not return an HTTP error after one Postgres-backed business write already
committed successfully just because the trailing remote-change observation
probe failed.

## Feature paths

- `none`

## Execution lanes

- `integration`

## Owned paths

- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W157-S01` | done | preserve truthful success responses after committed writes even when the trailing watermark observation fails, and cover it with a real Postgres regression | `./scripts/check-wave.sh --wave W157-post-success-write-veracity` |

## Language impact

`none`

## Invariant impact

`I2, I3, I9, I11`

## ADR impact

`none`
