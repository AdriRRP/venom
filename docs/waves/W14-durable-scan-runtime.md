# W14. Durable Scan Runtime

Wave: `W14-durable-scan-runtime`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `none`

## Goal

Add a minimal durable scan queue that executes canonical scan requests with explicit terminal status instead of hidden background behavior.

## Feature paths

- `features/request-scan.feature`

## Execution lanes

- `unit`
- `acceptance`

## Owned paths

- `docs/product-direction.md`
- `docs/ubiquitous-language.md`
- `docs/waves/W14-durable-scan-runtime.md`
- `features/request-scan.feature`
- `crates/venom-domain/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W14-S01` | done | add a durable queue for scan commands and record explicit completed or failed terminal states | `cargo test --workspace --all-targets --all-features`, `./scripts/check-acceptance.sh`, `./scripts/check-wave.sh --wave W14-durable-scan-runtime` |

## Language impact

`add`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- the runtime is intentionally single-threaded and explicit for now
- provider failures become durable failed commands rather than silent retries
