# W01. Foundation

Wave: `W01-foundation`
Status: `done`
BDD impact: `none`
Agentic impact: `docs`
Infra profile: `none`

## Goal

Bootstrap the repository so the wave/slice workflow, repo layout, and Rust workspace exist concretely before domain implementation begins.

## Feature paths

- `none`

## Execution lanes

- `none`

## Owned paths

- `README.md`
- `Cargo.toml`
- `.gitignore`
- `apps/api/...`
- `crates/venom-vulnerability-management/...`
- `docs/repo-structure.md`
- `docs/adr/0001-repo-structure-and-workspace-layout.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W01-S01` | done | initialize git and root hygiene | `git status`, repo initialized |
| `W01-S02` | done | define repo structure and record decision | docs present and coherent |
| `W01-S03` | done | bootstrap minimal Cargo workspace | `cargo check` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`0001-repo-structure-and-workspace-layout`

## Notes

- this wave intentionally has no canonical business BDD changes
- repository initialized as git and minimal Cargo workspace verified with `cargo check`
