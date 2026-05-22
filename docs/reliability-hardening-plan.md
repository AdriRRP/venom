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
5. `W93-bulk-governance-cohort-veracity`
6. `W94-local-collection-scan-materialization-atomicity`
7. `W95-local-scan-outcome-atomicity`
8. `W96-local-collection-scan-request-atomicity`
9. `W97-postgres-post-commit-application-hardening`
10. `W98-api-lock-and-snapshot-topology`
11. `W99-contextual-risk-semantic-completion`
12. `W100-bulk-cohort-streaming`
    Remove avoidable full-vector materialization and ordering from large
    bulk-governance cohorts.
13. `W101-postgres-governance-event-atomicity`
    Persist governance writes and their operator-facing system events in the
    same Postgres transaction.
14. `W102-api-read-snapshot-arc-sharing`
    Reuse unchanged HTTP read-snapshot lanes through `Arc` when refreshing one
    lane at a time.
15. `W103-postgres-integration-publication-event-atomicity`
    Persist integration publication outcome updates and their operator-facing
    system events in one Postgres transaction.
16. `W104-postgres-scan-request-event-atomicity`
    Persist Postgres scan-command enqueue writes and their operator-facing
    system events in one transaction.
17. `W105-http-worker-lock-splitting`
    Move the API service through an explicit slot so HTTP handlers no longer
    hold the global service mutex across awaited worker or mutation work.
18. `W106-postgres-read-snapshot-caches`
    Cache Postgres-backed `system events` and `command statuses` snapshot lanes
    and refresh them only on real mutation paths or rebuild.
19. `W107-bulk-governance-state-compaction`
    Remove full `FindingGovernance` and `FindingReadModel` clones from bulk
    governance actions in local and Postgres paths.

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
