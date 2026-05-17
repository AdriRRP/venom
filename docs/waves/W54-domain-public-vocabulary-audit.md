# W54. Domain Public Vocabulary Audit

Wave: `W54-domain-public-vocabulary-audit`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Tighten the public vocabulary of `venom-domain` so that crate-root exports expose stable domain concepts, while provider-contract helpers and Syft/Grype adapter details stay under their capability modules.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `crates/venom-domain/src/lib.rs`
- `crates/venom-domain/src/scanning/mod.rs`
- `crates/venom-domain/src/findings/mod.rs`
- `crates/venom-domain/src/scanning/syft_grype.rs`
- `crates/venom-domain/examples/contracts.rs`
- `crates/venom-domain/examples/syft_grype_live.rs`
- `apps/api/src/infra/postgres_backend.rs`
- `docs/waves/W54-domain-public-vocabulary-audit.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W54-S01` | done | remove non-canonical helpers and provider-specific adapter details from `venom-domain` crate-root exports and update call sites to use precise module paths | `scripts/check-slice.sh --wave W54-domain-public-vocabulary-audit --slice W54-S01 --lane integration --path crates/venom-domain/src/lib.rs --path crates/venom-domain/src/scanning/mod.rs --path crates/venom-domain/src/findings/mod.rs --path crates/venom-domain/src/scanning/syft_grype.rs --path crates/venom-domain/src/scanning/durable_scan_runtime.rs --path crates/venom-domain/src/scanning/scan_execution.rs --path crates/venom-domain/examples/contracts.rs --path crates/venom-domain/examples/syft_grype_live.rs --path apps/api/src/infra/postgres_backend.rs` |
| `W54-S02` | planned | close the wave with docs and full gate alignment | `scripts/check-wave.sh --wave W54-domain-public-vocabulary-audit` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep the crate root focused on stable domain nouns and role-facing results
