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
20. `W108-local-scheduler-materialization-veracity`
    Keep local collection scheduling side-effect free until durable
    materialization succeeds, and derive pending-due state from real durable
    inventory instead of planning clones.
21. `W109-contextual-risk-model-enrichment`
    Replace the flat context-pressure sum with deterministic contextual
    postures that distinguish public-edge, critical-internal, and hardened
    private workloads without losing predictability.
22. `W110-local-mutation-partial-progress-veracity`
    Refresh operator read snapshots even after failed local mutations so partial
    durable progress is visible instead of hidden behind stale HTTP views.
23. `W111-http-mutation-lane-parallelism`
    Reduce time spent inside the single-flight HTTP mutation slot by making
    volatile read lanes cheaper to refresh.
24. `W112-local-read-snapshot-volatile-lane-caches`
    Cache local `system events` and `command statuses` as `Arc` lanes inside the
    durable local stores.
25. `W113-system-event-window-semantics`
    Stop truncating recent system events before filtering so operator timelines
    and totals mean the same thing across local and Postgres backends.
26. `W114-postgres-bulk-governance-batching`
    Batch Postgres risk-acceptance and suppression upserts for collection and
    tag cohorts instead of issuing one write per finding.
27. `W115-contextual-risk-explainability`
    Expose the deterministic contextual posture behind each contextual risk
    decision to API and UI consumers.
28. `W116-http-mutation-lane-splitting`
    Restore the mutable HTTP application slot before publishing refreshed
    snapshots and add an explicit inspect path for truthful read-only queries.
29. `W117-local-read-side-deep-clone-elision`
    Cache local and Postgres-backed inventory and read-model lanes as shared
    `Arc` snapshots instead of cloning those structures on demand.
30. `W118-bounded-system-event-snapshots-with-truthful-querying`
    Keep `system events` snapshot lanes bounded while serving operator queries
    from the full local or Postgres-backed durable history.
31. `W119-local-partial-progress-contracts`
    Surface partial local scheduler progress explicitly when scan commands are
    already durable but schedule materialization metadata fails afterward.
32. `W120-contextual-risk-rule-explainability`
    Expose the exact deterministic contextual-risk rule that produced each
    operator-facing risk result.

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
