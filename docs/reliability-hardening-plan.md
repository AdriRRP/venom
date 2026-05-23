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
33. `W121-http-mutation-multi-lane`
    Move `system events` reads fully onto HTTP read snapshots so they no longer
    contend on the mutable application slot.
34. `W122-read-side-structural-sharing`
    Reuse cached release boards and shared read lanes instead of rebuilding
    those projections inside HTTP snapshots.
35. `W123-bounded-event-retention-and-indexed-querying`
    Replace in-memory `system events` queues with a bounded query index that
    preserves truthful totals and recent-category windows.
36. `W124-bulk-cohort-stream-compaction`
    Compute bulk-governance cohorts in one pass and stop materializing full
    matched vectors before filtering for actual state changes.
37. `W125-context-factor-explainability`
    Expose the exact effective context factors that shaped each deterministic
    contextual-risk result.
38. `W126-http-write-plane-critical-section-reduction`
    Rebuild refreshed HTTP snapshots after restoring the mutable application
    slot instead of while exclusive ownership is still held.
39. `W127-system-event-refresh-elision`
    Stop rebuilding inventory and findings lanes when a mutation only changes
    recent operator-facing system events.
40. `W128-collection-scan-scheduler-borrowed-inventory`
    Plan due collection scans from borrowed inventory instead of cloning the
    full inventory shape on each scheduler pass.
41. `W129-system-event-index-arc-compaction`
    Keep bounded recent system-event windows as shared `Arc<SystemEvent>`
    entries instead of duplicating full values per retained list.
42. `W130-context-factor-provenance`
    Expose factor-level contextual provenance so operators can see which scope
    supplied each effective trait.
43. `W131-http-write-plane-lane-partitioning`
    Drain long-running HTTP worker and publication loops one durable step at a
    time so other write operations can interleave between iterations.
44. `W132-incremental-read-side-refresh`
    Refresh inventory-backed and findings-backed snapshot lanes separately so
    governance and ingestion writes do not deep-clone unrelated read-side data.
45. `W133-postgres-remote-change-watermark`
    Refresh Postgres-backed API snapshots when another instance advanced the
    durable store, using one remote change watermark rather than blind process
    cache trust.
46. `W134-postgres-system-event-window-shaping`
    Rebuild Postgres operator event timelines from truthful totals plus recent
    windows instead of loading the whole system-event table into memory.
47. `W135-context-factor-identity-explainability`
    Expose the exact identity behind each effective context factor, not only
    whether it came from component, tag, or collection scope.

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
