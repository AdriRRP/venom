# W27. Integration Runtime and HTTP Publisher

Wave: `W27-integration-runtime-and-http-publisher`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Make integration publication runtime-configurable and durable, then add one real HTTP publisher adapter so the outbox loop can drive external delivery without ad hoc publisher selection in the worker request.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `crates/venom-domain/**`
- `apps/api/**`
- `docs/waves/W27-integration-runtime-and-http-publisher.md`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W27-S01` | in_progress | define the wave and add the missing domain term for durable integration runtime configuration | `./scripts/check-slice.sh --wave W27-integration-runtime-and-http-publisher --slice W27-S01 --path docs/waves/ACTIVE --path docs/waves/W27-integration-runtime-and-http-publisher.md --path docs/ubiquitous-language.md` |
| `W27-S02` | in_progress | add durable integration runtime configuration, remove ad hoc publisher selection from the worker path, and add one bounded HTTP publisher adapter with explicit timeout and status-code failure semantics | `cargo test --workspace --all-targets --all-features`, `./scripts/check-slice.sh --wave W27-integration-runtime-and-http-publisher --slice W27-S02 --lane integration` |
| `W27-S03` | planned | prove durable integration runtime reload and HTTP publication through Postgres rehearsal and the full wave gate | `./scripts/rehearse-infra.sh --profile db`, `./scripts/check-wave.sh --wave W27-integration-runtime-and-http-publisher` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep integration runtime configuration system-level, not per component
- keep the HTTP publisher bounded by explicit timeout and no hidden retries
- keep fixture-only failure simulation out of the durable runtime model
