# W198. Governance And Tracker Arc Sharing

Wave: `W198-governance-and-tracker-arc-sharing`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Reduce residual resident-state duplication across forked live API lanes by
keeping `FindingTracker` snapshots and `FindingGovernance` decisions under
copy-on-write `Arc` ownership instead of cloning their full maps on lane fork.

## Feature paths

- `crates/venom-domain/src/findings/finding_tracker.rs`
- `crates/venom-domain/src/findings/finding_governance.rs`

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/findings/finding_tracker.rs`
- `crates/venom-domain/src/findings/finding_governance.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W198-S01` | done | keep tracker snapshots and governance decisions under copy-on-write shared ownership across lane forks | `cargo test -p venom-domain clone_uses_copy_on_write_for --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
