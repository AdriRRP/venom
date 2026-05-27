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
48. `W136-postgres-write-path-remote-refresh`
    Refresh Postgres-backed API write paths from durable state before taking
    mutable business decisions, so multi-instance writes do not act on stale
    in-process projections.
49. `W137-scoped-active-findings-stream-compaction`
    Keep one bounded top-k window for active-findings pages instead of
    materializing and sorting every matching finding in one release-scoped or
    artifact-scoped query.
50. `W138-http-write-plane-partitioning`
    Let fresh Postgres-backed HTTP reads check a shared remote-change probe
    before they contend on the mutable application slot.
51. `W139-deep-read-side-clone-elision`
    Update command-status snapshot lanes incrementally on hot-path command
    transitions instead of rebuilding whole maps each time.
52. `W140-postgres-remote-refresh-delta-shaping`
    Reuse the merged local `system events` lane while the underlying state and
    runtime source arcs are unchanged.
53. `W141-system-event-index-cost-compaction`
    Stream matched bulk-governance findings directly into caller-owned durable
    write shapes instead of building extra intermediate vectors.
54. `W142-bulk-cohort-streaming-and-context-explainability`
    Render contextual rule and provenance data in operator UI as readable
    explanations rather than debug-shaped strings.
55. `W143-postgres-system-event-window-batching`
    Rebuild Postgres recent system-event windows from one ranked recent-events
    query plus truthful totals instead of one recent query per category.
56. `W144-collection-governance-overview-bounded-page`
    Keep release-governance pages bounded to the requested offset window while
    preserving whole-scope health and cohort totals.
57. `W145-structured-context-explainability`
    Render contextual profile, posture, rule, and effective factors as
    structured operator-facing UI content instead of one dense string.
58. `W146-http-remote-refresh-stampede-control`
    Let one stale-read refresh path perform the remote Postgres refresh while
    concurrent fresh readers reuse the result instead of stampeding the mutable
    application slot.
59. `W147-postgres-system-event-arc-window-sharing`
    Share one `Arc<SystemEvent>` across the global and per-category recent
    Postgres windows instead of duplicating one owned event value per window.
60. `W148-context-explainability-operator-layout`
    Present contextual summary, posture, rule, and effective factors in one
    more structured operator layout instead of plain stacked text.
61. `W149-http-write-plane-real-partitioning`
    Let stale Postgres-backed fresh reads rebuild one detached read snapshot
    without taking the live mutable application slot.
62. `W150-schema-scoped-postgres-remote-refresh`
    Detect remote Postgres changes from one VENOM schema-local watermark instead
    of the database-global WAL head.
63. `W151-remote-read-snapshot-watermark-reuse`
    Reuse one detached remote read snapshot until the schema-local watermark
    changes again instead of rebuilding on every fresh read while the main write
    store stays stale.
64. `W152-event-trace-index-shape-compaction`
    Merge bounded recent system-event windows directly instead of chain-sort-
    truncate rebuilds on every local composite refresh.
65. `W153-bulk-cohort-streaming-paths`
    Make visitor-based streaming paths the canonical bulk-governance API and
    remove leftover vector-materializing helpers from the read model surface.
66. `W154-remote-read-snapshot-monotonicity`
    Publish detached Postgres-backed fresh-read snapshots only when their
    change watermark is newer than the snapshot already visible to operators.
67. `W155-fresh-read-watermark-fast-path`
    Reuse one already-published detached Postgres read snapshot without taking
    the remote refresh lane when the schema-local watermark has not advanced.
68. `W156-api-service-slot-restoration-hardening`
    Always restore the live `ApiApplication` slot before returning errors from
    refresh or watermark-observation paths, and publish the freshest visible
    snapshot after successful writes or remote refreshes.
69. `W157-post-success-write-veracity`
    Keep HTTP write responses truthful when one Postgres-backed mutation already
    committed but the trailing remote-change observation probe fails afterward.
70. `W158-detached-read-watermark-promotion`
    Promote the shared observed Postgres change watermark after one detached
    fresh read succeeds so the next live write does not pay the same remote
    refresh again.
71. `W159-system-event-page-arc-sharing`
    Keep indexed `system events` queries on shared `Arc<SystemEvent>` entries
    until the API DTO projection layer instead of cloning full events
    immediately on every query.
72. `W160-command-status-rebuild-elision`
    Rebuild `command statuses` snapshot lanes incrementally during local replay
    and Postgres reload instead of regenerating full maps after each rebuild.
73. `W161-system-event-source-arc-sharing`
    Keep the live local and Postgres `system events` index as the shared
    snapshot `Arc` itself instead of cloning the whole index on each push.
74. `W180-system-event-category-window-truth`
    Keep one retained operator event store while restoring truthful recent
    category pages instead of filtering only the global recent window.
75. `W181-postgres-lane-bootstrap-forking`
    Open one rebuilt Postgres-backed application view and fork the other API
    lanes from that bootstrapped state instead of rebuilding all three.
76. `W182-read-model-inner-source-sharing`
    Move `FindingReadModel` internals onto copy-on-write `Arc` maps so remote
    refreshes and cloned lanes do not deep-clone active findings and decisions
    by default.
77. `W183-runtime-worker-barrier-narrowing`
    Reserve the state/runtime consistency barrier for true state writes and let
    Postgres runtime workers revalidate against durable state instead of taking
    the broad read barrier on every step.
74. `W176-shared-postgres-pool-across-api-lanes`
    Reuse one `PgPool` across the partitioned Postgres-backed API lanes instead
    of opening three independent pools for the same schema and process.
75. `W177-runtime-worker-state-barrier-narrowing`
    Let the scan-command worker lane avoid the state read barrier where it can
    refresh to the latest durable state immediately before applying outcomes.
76. `W178-inventory-core-remote-refresh-narrowing`
    Refresh only the changed inventory-core durable tables during detached
    Postgres reads instead of reloading the whole inventory-core subgraph.
77. `W179-single-window-system-event-categories`
    Derive category-scoped recent event views from one shared recent window
    instead of storing separate recent slot lists per category.
74. `W172-http-publication-lane-barrier-narrowing`
    Stop serializing publication writes behind the state/runtime consistency
    barrier when the publication lane does not depend on mutable state
    decisions.
75. `W173-local-tail-sync-stale-refresh`
    Replace local stale-lane reopen-and-replay with incremental tail replay
    from the durable JSONL histories.
76. `W174-postgres-sub-lane-refresh-narrowing`
    Split broad inventory/read-model remote refreshes into narrower detached
    sub-lane rebuilds that reuse current shared `Arc` sources.
77. `W175-system-event-slot-window-compaction`
    Keep retained recent `system events` in one shared slot store and let
    recent windows reference compact slot indexes instead of duplicating event
    ids per window.
74. `W162-api-health-degraded-observation`
    Surface committed-write remote-observation failures as degraded API health
    instead of silently preserving a healthy shell.
75. `W163-contextual-audit-surface`
    Turn contextual-risk explainability into a structured operator-facing audit
    surface in the findings UI.
76. `W164-inventory-source-arc-sharing`
    Keep the live inventory as the shared snapshot source arc instead of
    cloning the full inventory shape on each refresh.
77. `W165-read-model-source-arc-sharing`
    Keep the live finding read model as the shared snapshot source arc instead
    of cloning the full projection on each refresh.
78. `W166-http-write-lanes-and-remote-delta-refresh`
    Partition Postgres HTTP writes into state/runtime/publication lanes, make
    remote read refresh lane-aware, and remove eager release-board rebuilds.
79. `W167-lane-write-consistency-barrier`
    Prevent partitioned state/runtime/publication writes from operating on
    stale cached projections while another durable state mutation can still
    interleave.
80. `W168-change-journal-gap-fallback`
    Detect truncated change-journal gaps and fall back to a truthful full lane
    refresh instead of reusing stale cached arcs behind a newer watermark.
81. `W169-local-write-plane-partitioning`
    Partition the local HTTP write plane with correctness-preserving lane
    coordination so the file-backed profile is no longer forced through one
    global mutable slot.
82. `W170-remote-refresh-sub-lane-narrowing`
    Narrow detached Postgres refreshes below coarse inventory and read-model
    lanes so small remote changes do not reload unrelated tables.
83. `W171-system-event-merge-cost-compaction`
    Merge bounded recent system-event windows directly from retained ids and
    shared arcs instead of rebuilding merged windows through repeated queries.
74. `W162-api-health-degraded-observation`
    Surface post-write remote-watermark observation drift as explicit degraded
    API health instead of silently swallowing the operator signal.
75. `W163-contextual-audit-surface`
    Render contextual posture, rule, and effective factor provenance as a
    structured operator audit surface instead of mixed text fragments.
76. `W164-inventory-source-arc-sharing`
    Keep the live inventory under shared `Arc<ComponentInventory>` ownership so
    inventory snapshot refreshes reuse the source structure instead of deep
    cloning the full inventory on each relevant mutation.
77. `W165-read-model-source-arc-sharing`
    Keep the live `FindingReadModel` under shared `Arc` ownership so read-model
    snapshot refreshes reuse the source structure instead of cloning the full
    projection on each relevant mutation.
78. `W166-http-write-lanes-and-remote-delta-refresh`
    Partition the Postgres HTTP write plane by mutation lane, refresh remote
    state by changed snapshot lanes instead of detached full rebuilds, make the
    `ReleaseBoard` lazy over source arcs, and compact recent system-event
    indexing so retained events are stored once and referenced cheaply.

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
