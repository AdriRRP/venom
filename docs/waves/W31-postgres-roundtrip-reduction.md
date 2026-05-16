# W31. Postgres Round-Trip Reduction

Wave: `W31-postgres-roundtrip-reduction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `db`

## Goal

Reduce unnecessary Postgres write round trips in a hot durable business path without changing durable semantics or weakening explicit failure behavior.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W31-S01` | done | define the Postgres round-trip reduction wave and target | `./scripts/check-slice.sh --wave W31-postgres-roundtrip-reduction --slice W31-S01 --path docs/waves/ACTIVE --path docs/waves/W31-postgres-roundtrip-reduction.md` |
| `W31-S02` | done | batch integration outbox inserts inside Postgres transactions instead of inserting each event separately | `cargo test --workspace --all-targets --all-features && ./scripts/rehearse-infra.sh --profile db` |
| `W31-S03` | done | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W31-postgres-roundtrip-reduction` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- optimize only a measured durable path
- preserve explicit transaction boundaries and error visibility
