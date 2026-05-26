# W162. API Health Degraded Observation

Wave: `W162-api-health-degraded-observation`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Make post-write remote observation drift visible to operators instead of
silently preserving `healthy` API status after the business write already
succeeded.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `web`

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/web/src/app/app-shell.tsx`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/dashboard.tsx`
- `apps/web/src/routes/events.tsx`
- `apps/web/src/routes/findings.tsx`
- `apps/web/src/routes/operations.tsx`
- `apps/web/src/styles.css`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W162-S01` | done | degrade API health when remote watermark observation is stale after one durable write and clear it once alignment is re-established | `cargo test -p venom-api api_health_reports_degraded_when_remote_observation_is_stale --all-features`, `npm --prefix apps/web run check` |

## Language impact

`none`

## Invariant impact

`I2`, `I9`

## ADR impact

`none`

## Notes

`W157` fixed false error responses after successful durable writes, but it left
the operator shell blind to the follow-up probe failure. This wave keeps the
truthful success contract while surfacing the degraded coordination state
explicitly through `/health` and the shell.
