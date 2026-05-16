# W25. Outbox and Integration Events

Wave: `W25-outbox-and-integration-events`
Status: `in_progress`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Introduce a durable outbox and a bounded integration-event publication path early, so event-driven integration becomes a governed part of the core loop instead of a late unsafe add-on.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `contract`
- `infra`

## Owned paths

- `crates/venom-domain/**`
- `apps/api/**`
- `tests/contracts/integration-events/**`
- `infra/**`
- `scripts/**`
- `docs/adr/0003-durable-outbox-and-integration-events.md`
- `docs/waves/W25-outbox-and-integration-events.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W25-S01` | done | define the durable outbox boundary, external event contract, wave slices, and performance/reliability constraints before implementation starts | `./scripts/check-slice.sh --wave W25-outbox-and-integration-events --slice W25-S01 --path docs/waves/ACTIVE --path docs/waves/W25-outbox-and-integration-events.md --path docs/adr/0003-durable-outbox-and-integration-events.md --path docs/product-direction.md --path docs/ubiquitous-language.md --path tests/contracts/README.md --path tests/contracts/integration-events/README.md` |
| `W25-S02` | done | append pending canonical integration events durably together with provider-report and scan-command business writes | `cargo test --workspace --all-targets --all-features`, `./scripts/check-slice.sh --wave W25-outbox-and-integration-events --slice W25-S02 --lane integration` |
| `W25-S03` | planned | publish pending integration events in stable bounded batches and persist explicit publication outcome | `cargo test --workspace --all-targets --all-features`, `./scripts/check-slice.sh --wave W25-outbox-and-integration-events --slice W25-S03 --lane contract`, `./scripts/check-slice.sh --wave W25-outbox-and-integration-events --slice W25-S03 --lane integration` |
| `W25-S04` | planned | prove restart, replay, and duplicate-publication behavior through Postgres rehearsal and contract checks | `./scripts/check-contracts.sh`, `./scripts/rehearse-infra.sh --profile db`, `./scripts/check-wave.sh --wave W25-outbox-and-integration-events --lane infra` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`add`

## Notes

- start with the outbox as the durable publication boundary; do not make the broker the source of truth
- keep publication at-least-once externally and idempotent by durable event identity
- keep the first publisher path bounded by explicit batch size and explicit result persistence
- avoid per-event task fan-out, hidden retries, and unbounded in-memory buffering on durable paths
