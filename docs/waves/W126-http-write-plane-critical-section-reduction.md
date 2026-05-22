# W126. HTTP Write Plane Critical Section Reduction

Wave: `W126-http-write-plane-critical-section-reduction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce how long the HTTP mutation lane keeps exclusive ownership of the mutable
application service during snapshot refreshes.

## Owned paths

- `apps/api/src/http/mod.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W126-S01` | done | collect snapshot refresh lanes while holding the mutable service, then rebuild the published read snapshot after service ownership is restored | `unit`, `integration` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`
