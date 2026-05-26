# W168. Change Journal Gap Fallback

Wave: `W168-change-journal-gap-fallback`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Keep detached Postgres lane refresh truthful even when one instance falls
behind the retained `change_journal` window, by detecting the gap and forcing
safe broad refresh instead of reusing stale cached arcs.

## Feature paths

- `apps/api/src/infra/postgres_backend.rs`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/api/src/infra/postgres_backend.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W168-S01` | done | detect `change_journal` coverage gaps relative to `since_change_watermark` | `cargo check -p venom-api --all-features --offline` |
| `W168-S02` | done | fall back to safe broad lane reloads when the gap is detected and cover the decision with regression tests | `cargo test -p venom-api change_journal_gap --all-features --offline` |

## Language impact

`none`

## Invariant impact

`I8`, `I9`, `I11`

## ADR impact

`none`
