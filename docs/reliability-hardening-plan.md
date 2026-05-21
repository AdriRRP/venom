# Reliability Hardening Plan

## Purpose

Close the current post-feature architectural gaps that most directly threaten
veracity, efficiency, and operator trust.

This plan tracks corrective waves discovered from audit, not legacy debt.

## Execution order

Completed:

1. `W89-system-event-trace-veracity`
2. `W90-api-read-snapshot-compaction`
3. `W91-release-projection-consolidation`
4. `W92-gate-signal-hardening`

Remaining:

5. `W93-bulk-governance-cohort-veracity`
   Separate paged read queries from bulk-action cohort queries so bulk actions
   never silently operate on one page only.
6. `W94-local-collection-scan-materialization-atomicity`
   Remove the local two-store split between collection schedule materialization
   and durable command enqueue.
7. `W95-local-scan-outcome-atomicity`
   Remove the local split between durable finding mutation and durable scan
   command terminal outcome.

## Exit condition

This block is closed when:

- recent-event queries mean the same thing across local and Postgres backends
- API read-side refresh does not rebuild large structures more often than needed
- release boards and dashboards no longer multiply full-scope scans by view
- acceptance and browser gates fail with explicit, durable diagnostics rather
  than format-coupled heuristics
- bulk governance actions operate over their full matched cohort or fail
  explicitly
- local durable scheduler and command execution paths do not claim completion
  across split stores before all coordinated business writes are durable
