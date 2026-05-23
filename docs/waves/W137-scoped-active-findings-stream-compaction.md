# W137. Scoped Active Findings Stream Compaction

Wave: `W137-scoped-active-findings-stream-compaction`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Serve active-findings pages from one bounded top-k working set so release- and
artifact-scoped queries keep truthful totals without materializing and sorting
every matching finding in memory.

## Execution lanes

- `unit`

## Owned paths

- `crates/venom-domain/src/findings/finding_read_model.rs`
- `docs/reliability-hardening-plan.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W137-S01` | done | replace full-vector page building in active-findings queries with bounded top-k selection while preserving ordering and totals | `unit` |

## Language impact

`none`

## Invariant impact

`I8`, `I11`

## ADR impact

`none`
