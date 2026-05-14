# W02. CI Quality Gates

Wave: `W02-ci-quality-gates`
Status: `done`
BDD impact: `none`
Agentic impact: `script`
Infra profile: `none`

## Goal

Install the first repository-enforced CI gates by compressing the strongest legacy patterns into a smaller local-first setup.

## Feature paths

- `none`

## Execution lanes

- `none`

## Owned paths

- `.github/workflows/**`
- `scripts/**`
- `rust-toolchain.toml`
- `README.md`
- `docs/repo-structure.md`
- `docs/work-methodology.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W02-S01` | done | add stable local quality and security scripts | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all -W clippy::pedantic -W clippy::nursery -W clippy::perf -W clippy::cargo -A clippy::multiple_crate_versions`, `cargo test --workspace --all-targets --all-features`, `bash -n scripts/*.sh` |
| `W02-S02` | done | add GitHub Actions gates optimized from legacy | inspect workflow YAML and shell entrypoints |
| `W02-S03` | done | update minimal docs so CI belongs to the repo model | inspect updated docs |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- `unused-deps` is implemented from day one but should only become a required branch check after it proves stable across real waves
- local validation hit an outdated global `cargo-udeps` (`0.1.48`) that does not understand workspace `resolver = "3"`; CI installs the latest tool per run
