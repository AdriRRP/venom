# W161. System Event Source Arc Sharing

Wave: `W161-system-event-source-arc-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep the live local and Postgres `system events` index as the shared snapshot
`Arc` itself instead of cloning the whole index on each push.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/durable_state.rs`
- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W161-S01` | done | make the live `system events` index itself the shared snapshot source in local and Postgres stores | `./scripts/check-wave.sh --wave W161-system-event-source-arc-sharing` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
