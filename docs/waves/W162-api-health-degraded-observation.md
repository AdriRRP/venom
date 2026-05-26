# W162-api-health-degraded-observation

## Goal

Make post-write remote observation drift visible to operators instead of
silently preserving `healthy` API status after the business write already
succeeded.

## Why

`W157` made write responses truthful by returning success even when the trailing
remote watermark observation failed. That fixed false error responses, but it
left a quieter operator gap: the shell still showed the API as healthy even
though fresh remote alignment had not been re-established yet.

## Scope

- track remote observation degradation in HTTP API state
- expose degraded API health from `/health`
- surface degraded health in the web shell

## Out of scope

- full write-plane partitioning
- incremental Postgres detached refresh
- deeper structured contextual explainability

## Slices

### W162-S01

Status: done

- record remote observation degradation when a successful write cannot update
  the remote probe
- clear degradation when a later refresh or successful probe observation
  re-establishes alignment
- expose `degraded` from `/health`
- render degraded health explicitly in the shell

## Verification

- `cargo test -p venom-api api_health_reports_degraded_when_remote_observation_is_stale --all-features`
- `npm --prefix apps/web run check`
- `./scripts/check-wave.sh --wave W162-api-health-degraded-observation`

## Agentic impact

None.

## Documentation impact

- update the reliability hardening plan with the new corrective wave
