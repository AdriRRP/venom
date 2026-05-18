# Legacy Value Audit

## Purpose

Capture the highest-value product ideas from the legacy VENOM codebase without
re-importing its excess architectural weight, provider coupling, or operational
ceremony.

This audit is based on the legacy repository at:

- `/Volumes/media/Backup Stratio/cyber-venom/cyber-venom`

## What still looks high-value

### 1. Release-centric operations

The strongest product idea in the legacy system is not "findings in general",
but findings scoped to one release or one operational slice of the platform.

Legacy signals:

- collection aggregate and replace semantics
- collections page and collection detail page
- dashboard summaries scoped by collection
- findings filters that include `collectionId`
- collection-driven periodic refresh and membership orchestration

Why it matters now:

- the current VENOM already has closed release collections and periodic scan
  scheduling
- operators naturally work per release, platform slice, or owned scope
- release scope is the right unit for daily governance and reporting

### 2. Governance state machine over findings

The second strongest legacy capability is explicit operator governance over a
finding:

- classify
- suppress
- unsuppress
- risk accept
- false positive
- withdraw

Legacy signals:

- `FindingStateActions.tsx`
- dashboard summaries by state
- finding state filters and timelines
- legacy managed-vulnerability invariants around suppression and risk acceptance

Why it matters now:

- this is the shortest path from "scanner console" to "daily vulnerability
  management system"
- governance decisions are durable business actions with high operator value
- the current product direction already names this as a core missing capability

### 3. Execution context and contextual risk

The most differentiated idea in the legacy system is not raw severity, but
severity after runtime context is known.

Legacy signals:

- component execution context
- context profiles / presets
- deterministic CVSS contextualization
- component source repository linkage
- contextualizer profile import and propagation

Why it matters now:

- current VENOM already separates finding ingestion from product meaning
- contextual risk is how the product becomes more than a transport/orchestration
  layer
- deterministic contextualization fits the current "near embedded-grade" and
  "near nation-grade" goals much better than opaque AI-first logic

### 4. Release dashboards and operational summaries

The legacy UI invested heavily in operator-facing summaries:

- dashboard global summary
- dashboard per collection
- critical/high counts
- findings by state
- recent findings

Why it matters now:

- once governance exists, operators need compact release-level visibility, not
  only table queries
- these summaries compress large finding sets into actionable release status

### 5. Collection-driven bulk actions

The legacy system used collections as the fan-out unit for policy changes:

- assign preset to all components in one collection
- reconcile or replace membership
- source-driven collection updates

Why it matters now:

- collections already exist as the canonical release scope
- bulk operations let operators govern releases instead of clicking component by
  component
- this is one of the clearest leverage points in a platform setting

### 6. Source-driven collections

The legacy collection model distinguished:

- custom collections
- GitHub-driven collections with replace/reconcile semantics

Why it matters now:

- real platform releases are often materialized from repository-owned manifests
- automatic scope refresh is high-value once manual collections are stable
- this can be introduced cleanly after governance and dashboards exist

### 7. Event trace and operational observability

The legacy system had a useful operator idea around command/event visibility:

- system events page
- correlation and causation tracing
- recent commands
- operational alerts

Why it matters now:

- the current VENOM already has durable commands, workers, and outbox semantics
- once governance actions and scheduled runs grow, operators need traceability
- this is especially valuable for reliability-focused operations

## What should not be ported directly

### 1. Actor-heavy internal machinery

The legacy shared/application stack had a lot of:

- actor supervisors
- command buses
- event buses
- macros
- generic aggregate infrastructure

The current codebase should continue to avoid importing that weight unless a
concrete reliability bottleneck proves it necessary.

### 2. Provider-coupled default paths

The legacy system had strong Wiz-specific shape in parts of the scanner layer.
That should stay out of the current default path.

### 3. Migration / cutover / dual-runtime ceremony

A meaningful part of the legacy operational surface was migration-specific:

- cutover dashboards
- readiness gates for migration
- resilience scripts around old/new runtime modes

These were useful there but are not product value to port now.

### 4. Overgrown top-level UI breadth

The legacy UI had many pages. Not all of them were equally valuable. The
highest-value subset is:

- findings
- collections
- collection detail
- dashboard
- context profiles
- system events

Pages outside that should not be treated as automatic roadmap commitments.

## Best value sequence for current VENOM

These are the waves that best translate legacy value into the current product
shape.

### W59. Governance decisions for findings

Goal:

- make findings governable through durable operator decisions

Scope:

- domain: explicit finding decision model and invariants
- API: endpoints for classify / suppress / accept risk / false positive /
  withdraw
- UI: operator actions from findings and release-scoped views

Why first:

- highest direct product value
- current release collection and active findings work is already in place
- unlocks dashboards and reporting that mean something

### W60. Context profiles and component execution context

Goal:

- let operators define reusable context profiles and attach execution context to
  managed components

Scope:

- domain: component context and preset references
- API: CRUD for profiles and assignment to components
- UI: context profile console and component/collection bulk application

Why second:

- it provides the input required for contextual risk
- maps cleanly to the strongest differentiated legacy idea

### W61. Deterministic contextual risk scoring

Goal:

- derive contextual severity from deterministic execution context

Scope:

- domain: contextual scoring service and rationale
- API: expose base vs contextual severity and rationale
- UI: show original vs contextual meaning in findings and release views

Why third:

- this is where VENOM stops being just scan orchestration plus governance
- deterministic logic keeps trust and reproducibility high

### W62. Release dashboard and collection health overview

Goal:

- give operators a compact release-first status view

Scope:

- API: per-collection summaries, critical/high counts, findings by state,
  recent changes
- UI: release dashboard and collection overview cards

Why fourth:

- it converts existing data and governance into fast operational understanding

### W63. Bulk governance and context actions by collection

Goal:

- let operators apply context and governance actions across release scope

Scope:

- API + UI for collection-scoped bulk operations
- strict explicitness and no hidden fan-out retries

Why fifth:

- this is where collections become a true operator leverage point

### W64. Source-driven collections

Goal:

- materialize release scope from repository-owned manifests

Scope:

- collection source model
- replace/reconcile semantics
- periodic refresh

Why later:

- very valuable, but safer after manual collections and governance are mature

### W65. System event trace and operator observability

Goal:

- expose command/event traceability for scheduled scans, governance, and
  publication loops

Scope:

- command trace views
- recent operations
- explicit alerts over failed/stale work

Why later:

- extremely valuable for reliability, but strongest after more business actions
  exist

## Recommended near-term roadmap

If the goal is to maximize legacy value with the current architecture, the best
next block is:

1. `W59-governance-decisions-for-findings`
2. `W60-context-profiles-and-component-context`
3. `W61-deterministic-contextual-risk-scoring`
4. `W62-release-dashboard-and-collection-health`

That sequence preserves the strongest legacy product thesis while staying
aligned with the current system's simpler, more reliable core.
