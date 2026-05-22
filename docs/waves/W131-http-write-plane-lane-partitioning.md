# W131. HTTP Write-Plane Lane Partitioning

Wave: `W131-http-write-plane-lane-partitioning`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Shorten the exclusive HTTP write slot for long worker-style loops by draining
one durable step at a time instead of holding the mutable application service
for the whole loop.

## Owned paths

- `apps/api/src/http/mod.rs`
- `apps/api/src/app/service.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W131-S01` | done | stepwise-drain collection scans, scan workers, and integration publication through the HTTP write slot | `unit`, `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`
