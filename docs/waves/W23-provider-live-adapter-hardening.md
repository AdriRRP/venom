# W23. Provider Live Adapter Hardening

Wave: `W23-provider-live-adapter-hardening`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `full`

## Goal

Harden the live Syft + Grype adapter with explicit execution limits so live provider failures stay bounded, explicit, and retryable.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `crates/venom-domain/Cargo.toml`
- `crates/venom-domain/src/syft_grype.rs`
- `scripts/infra-smoke.sh`
- `docs/waves/W23-provider-live-adapter-hardening.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W23-S01` | done | add explicit live command timeouts and bounded stderr/error shaping to the docker-backed Syft + Grype provider, and make the `full` infra lane actually execute the live adapter path | `cargo test --workspace --all-targets --all-features`, `./scripts/rehearse-infra.sh --profile full` |
| `W23-S02` | done | reset the active wave pointer after closing the live adapter hardening wave | `./scripts/check-wave.sh --wave W23-provider-live-adapter-hardening` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- live provider execution must time out explicitly instead of waiting without bound
- provider process stderr must be bounded so hot-path failure handling stays compact
