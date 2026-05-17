# W57. Import Path Compaction

Wave: `W57-import-path-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `compact`
Infra profile: `none`

## Goal

Compact import paths after the naming changes by moving major consumers to capability-scoped `venom_domain` imports instead of the broader crate-root surface.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/app/service.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `crates/venom-domain/examples/**`
- `crates/venom-domain/benches/hot_paths.rs`
- `docs/waves/W57-import-path-compaction.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W57-S01` | done | move major consumers to capability-scoped imports and reduce dependence on the broader crate-root surface | `scripts/check-slice.sh --wave W57-import-path-compaction --slice W57-S01 --lane integration --path apps/api/src/app/service.rs --path apps/api/src/infra/postgres_backend.rs --path crates/venom-domain/examples --path crates/venom-domain/benches/hot_paths.rs` |
| `W57-S02` | done | close the wave with docs and full gate alignment | `scripts/check-wave.sh --wave W57-import-path-compaction` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- prefer `venom_domain::findings`, `inventory`, `integration`, and `scanning` imports over one large root import block
