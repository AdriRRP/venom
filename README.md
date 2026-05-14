# VENOM

VENOM is being rebuilt as a provider-agnostic vulnerability management platform with a workflow that is deterministic for both humans and agents.

## Start Here

- [CONTRIBUTING.md](/Users/adrianramos/Cybersecurity/Venom/CONTRIBUTING.md)
- [docs/waves/ACTIVE](/Users/adrianramos/Cybersecurity/Venom/docs/waves/ACTIVE)
- [docs/product-direction.md](/Users/adrianramos/Cybersecurity/Venom/docs/product-direction.md)
- [docs/work-methodology.md](/Users/adrianramos/Cybersecurity/Venom/docs/work-methodology.md)
- [docs/repo-structure.md](/Users/adrianramos/Cybersecurity/Venom/docs/repo-structure.md)

## Current Repository Shape

- `.github/workflows/`: required and scheduled CI gates
- `apps/`: runtime entrypoints
- `crates/`: Rust libraries and bounded-context code
- `docs/`: project documentation
- `features/`: canonical executable specifications
- `tests/contracts/`: port and adapter compatibility checks
- `scripts/`: deterministic automations
- `fixtures/`: reusable local test data
- `infra/`: local infrastructure assets
