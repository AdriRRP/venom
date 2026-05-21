# Reliability Hardening Plan

## Purpose

Close the current post-feature architectural gaps that most directly threaten
veracity, efficiency, and operator trust.

This plan tracks corrective waves discovered from audit, not legacy debt.

## Execution order

1. `W89-system-event-trace-veracity`
   Make operator-facing system events truthful and backend-consistent.
2. `W90-api-read-snapshot-compaction`
   Reduce full-structure cloning in API read snapshots and refresh paths.
3. `W91-release-projection-consolidation`
   Consolidate release-scoped health and dashboard reads into cheaper dedicated
   projections.
4. `W92-gate-signal-hardening`
   Replace fragile text-parsing gate signals where practical and preserve
   failure diagnostics.

## Exit condition

This block is closed when:

- recent-event queries mean the same thing across local and Postgres backends
- API read-side refresh does not rebuild large structures more often than needed
- release boards and dashboards no longer multiply full-scope scans by view
- acceptance and browser gates fail with explicit, durable diagnostics rather
  than format-coupled heuristics
