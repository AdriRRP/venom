# W26. Integration Publication Worker Loop

Wave: `W26-integration-publication-worker-loop`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Connect the durable outbox to the application runtime through one bounded publication worker loop, so operators can publish pending integration events explicitly and safely over both local and Postgres backends.

## Feature paths

- `none`

## Execution lanes

- `integration`
- `contract`
- `infra`

## Owned paths

- `apps/api/**`
- `docs/waves/W26-integration-publication-worker-loop.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W26-S01` | done | define the app-level publication worker loop boundary and verification scope before changing runtime code | `./scripts/check-slice.sh --wave W26-integration-publication-worker-loop --slice W26-S01 --path docs/waves/ACTIVE --path docs/waves/W26-integration-publication-worker-loop.md` |
| `W26-S02` | done | add a bounded integration publication loop to the app service and API over local and Postgres durable backends | `cargo test --workspace --all-targets --all-features`, `./scripts/check-slice.sh --wave W26-integration-publication-worker-loop --slice W26-S02 --lane integration` |
| `W26-S03` | done | prove bounded publication and durable published-state reload through Postgres rehearsal | `./scripts/rehearse-infra.sh --profile db`, `./scripts/check-wave.sh --wave W26-integration-publication-worker-loop --lane infra` |
| `W26-S04` | done | align Postgres scan completion publication with the local runtime so both durable paths emit the same canonical event set | `./scripts/check-quality.sh`, `cargo test -p venom-api postgres_ -- --nocapture` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep publication bounded by explicit batch size
- keep local and Postgres execution paths behaviorally aligned
- do not hide publication failure behind retries inside the app worker loop
