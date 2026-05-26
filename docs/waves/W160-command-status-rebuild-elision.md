# W160. Command Status Rebuild Elision

Wave: `W160-command-status-rebuild-elision`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Rebuild `command statuses` snapshot lanes incrementally during local replay and
Postgres reload instead of regenerating whole maps after each rebuild.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/scanning/durable_scan_runtime.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W160-S01` | done | populate `command statuses` caches incrementally during replay/reload and remove redundant full rebuild refreshes | `./scripts/check-wave.sh --wave W160-command-status-rebuild-elision` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
