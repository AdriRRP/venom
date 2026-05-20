# Product Direction

## Mission

Turn raw vulnerability scanner output into contextual, operational decisions on real software components.

The platform target is:

- near embedded-grade efficiency in steady-state execution
- near nation-grade reliability for durable business paths

## Product outcomes

- know which components and collections are under management
- define closed release collections as canonical scan scope and schedule periodic scans over them
- derive one managed release collection from one declared source with explicit replace or reconcile semantics
- apply one managed context profile across one closed release collection without component-by-component operator fan-out
- query active findings over one closed release collection without reconstructing scope by hand
- work governed findings in one release-scoped workbench without losing the collection health context
- see one explicit bulk-governance cohort summary before acting over one filtered open release scope
- apply one explicit governance action to a filtered open cohort inside one release collection
- see one executive release dashboard that compresses schedule state, collection health, governed findings, and elevated contextual risk
- see one compact health summary for every managed release collection
- see one deterministic contextual risk level derived from execution context on active findings views
- see one recent operator-facing system event timeline across scheduler, command, governance, and publication activity
- reopen governed findings back to the canonical open state without losing durable traceability
- see which release collections are due now, which are scheduled next, and what the last scheduler pass materialized
- ingest provider scan reports from multiple providers or local fixtures
- separate canonical vulnerabilities from component-specific findings
- derive discovery and withdrawal semantics inside VENOM rather than trusting provider delta semantics
- classify findings using execution context and governance decisions
- expose durable operational views that survive restarts and infrastructure faults
- queue scan execution durably and expose explicit terminal command state
- publish durable integration events without unsafe dual writes
- keep hot paths, memory use, and infrastructure chatter aggressively lean

## Capability map

| Capability | Why it exists | Typical canonical feature shape |
|---|---|---|
| Inventory | know what is under management and how release scope is grouped | `register-component.feature`, `manage-collections.feature` |
| Scan orchestration | express canonical scan requests over managed ownership and closed collections | `request-scan.feature`, `request-collection-scan.feature`, `schedule-collection-scan.feature` |
| Finding ingestion | import concrete provider observations over immutable artifacts | `report-finding.feature` |
| Durable operations | rebuild active findings and durable scan command state after reload | `view-active-findings.feature`, `request-scan.feature` |
| Integration publication | expose durable domain changes to external consumers safely | `tests/contracts/integration-events/**` |
| Contextual risk | change meaning by runtime context | `classify-finding.feature` |
| Governance | accept, suppress, reopen, explain | `accept-risk.feature`, `suppress-finding.feature`, `reopen-finding.feature` |
| Operations | answer what is active, changed, pending, and due now | `view-active-findings.feature`, `view-collection-schedules.feature` |
| Reliability substrate | keep all of the above durable and rebuildable | usually verified by infra and acceptance gates rather than a standalone business feature |

## Wave discovery rule

Use this file when `docs/waves/ACTIVE` is `NONE` or when the current wave is about to close.

Choose the next wave by crossing:

1. one missing capability or one reliability gap
2. one observable outcome for a user or operator
3. one session-sized boundary that can end with a green wave gate

Default priority:

1. broken or unsafe behavior
2. missing capability on the critical product path
3. infrastructure risk that can invalidate the next capability
4. observability or operator clarity
5. convenience or ergonomics

Prefer a pure infrastructure wave only when:

- it unblocks the next capability; or
- it closes a reliability risk that would make the next capability misleading or unsafe

A wave is too large if at least one is true:

- it changes more than one primary capability
- it needs unrelated BDD features
- it needs both new business behavior and a large infrastructure redesign
- it cannot name one dominant verification story
