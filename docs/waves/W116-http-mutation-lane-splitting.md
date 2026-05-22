# W116. HTTP Mutation Lane Splitting

Wave: `W116-http-mutation-lane-splitting`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Shorten the time the HTTP mutation plane owns the mutable application slot by
building refreshed read snapshots outside the restore-and-publish boundary.

## Owned paths

- `apps/api/src/http/mod.rs`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W116-S01` | done | restore the mutable `ApiApplication` slot before publishing refreshed read snapshots, and add an `inspect` path for truthful read-only queries that must bypass bounded snapshots | `integration`, `web` |

## Invariant impact

`I8`, `I11`
