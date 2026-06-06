# W227. Local Forked Volatile Lane Parity

Wave: `W227-local-forked-volatile-lane-parity`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Remove the legacy local volatile-lane reopen-from-disk path and make local
`runtime` and `publication` lanes follow the same ephemeral fork-from-state
residency model already hardened for Postgres, so both backends share one
truthful volatile-lane topology instead of drifting into backend-specific
correctness fixes.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `apps/api/src/http/mod.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W227-S01` | done | switch local volatile lanes from disk reopen residency to ephemeral fork-from-state residency | `cargo test -p venom-api local_runtime_lane_is_ephemeral_until_taken --all-features --offline` |
| `W227-S02` | done | remove reopen-only local ephemeral probes and keep tests aligned to the unified fork model | `cargo test -p venom-api local_publication_lane_is_ephemeral_until_taken --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
