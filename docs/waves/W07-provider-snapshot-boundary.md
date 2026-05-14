# W07. Provider Snapshot Boundary

Wave: `W07-provider-snapshot-boundary`
Status: `done`
BDD impact: `create`
Agentic impact: `script`
Infra profile: `full`

## Goal

Define the canonical provider boundary so VENOM can ingest real findings from `Syft + Grype` without coupling the domain to scanner-specific payloads or lifecycle semantics.

## Feature paths

- `features/report-finding.feature`

## Execution lanes

- `unit`
- `integration`
- `infra`
- `acceptance`
- `contract`

## Owned paths

- `docs/product-direction.md`
- `docs/ubiquitous-language.md`
- `docs/architecture-invariants.md`
- `docs/adr/0002-sbom-first-provider-snapshot-boundary.md`
- `docs/waves/W07-provider-snapshot-boundary.md`
- `.github/workflows/dependency-freshness.yml`
- `scripts/check-dependency-freshness.sh`
- `scripts/check-contracts.sh`
- `tests/contracts/**`
- `crates/venom-domain/**`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W07-S01` | done | define the provider boundary, language, and architectural rule for canonical provider scan reports | inspect updated docs and `cargo check` |
| `W07-S02` | done | add the first canonical business feature for reporting findings from provider snapshots and wire the acceptance lane | `./scripts/check-acceptance.sh`, `./scripts/check-slice.sh --wave W07-provider-snapshot-boundary --slice W07-S02 --lane acceptance` |
| `W07-S03` | done | implement the first provider contract checks for deterministic and live provider-mode compatibility | `./scripts/check-contracts.sh`, `./scripts/check-slice.sh --wave W07-provider-snapshot-boundary --slice W07-S03 --lane contract` |
| `W07-S04` | done | add the first `Syft + Grype` adapter path and fixture corpus | `./scripts/check-contracts.sh`, `./scripts/rehearse-infra.sh --profile full`, `cargo test --workspace --all-targets --all-features` |
| `W07-S05` | done | add an advisory dependency freshness gate aligned with the legacy non-breaking update check | `./scripts/check-dependency-freshness.sh`, workflow inspection |

## Language impact

`add`

## Invariant impact

`I10 VENOM owns finding lifecycle semantics`

## ADR impact

`0002-sbom-first-provider-snapshot-boundary`

## Notes

- do not let provider-specific ids, severity names, or webhook event names leak into the core vocabulary
- the committed fixture corpus uses `alpine:3.21` and official scanner releases to avoid EOL alert noise while keeping real payload shape
