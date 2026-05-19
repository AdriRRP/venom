# Debt Closure Plan

## Purpose

Close the remaining high-value product debt carried over from the legacy VENOM
audit through one finite sequence of small, verifiable waves.

This plan only tracks debt that is still materially missing from the current
system. It does not list open-ended future enhancements.

## Completed debt already absorbed

The current system already closed the following legacy-value gaps:

- closed release collections as the canonical operator scope
- periodic collection scanning
- release-scoped active findings and governance workbench
- release dashboard and collection health summaries
- explicit finding governance for risk acceptance and suppression
- deterministic contextual risk from managed context profiles
- first bulk governance actions over one release-scoped open cohort

## Remaining debt to close

### W80. Bulk governance workbench

Goal:

- turn the current pair of collection-scoped bulk actions into one explicit
  operator workbench with one scoped cohort summary and one consistent action
  flow

Closes:

- the partial state of legacy bulk governance by collection

### W81. Source-driven collections

Goal:

- let one managed collection derive membership from one declared source with
  explicit replace or reconcile semantics

Closes:

- the largest remaining release-scope automation gap from the legacy system

### W82. System event trace and operator observability

Goal:

- expose command, scheduler, governance, and publication traceability as one
  operator-facing event timeline

Closes:

- the largest remaining observability gap from the legacy system

### W83. Governance decision lifecycle completion

Goal:

- complete the missing decision lifecycle around governed findings, including
  reversal or expiry-oriented operator flows where the domain meaning justifies
  them

Closes:

- the remaining gap between basic governance and a full daily-use governance
  lifecycle

### W84. Collection-scoped context actions

Goal:

- apply managed context profiles across one closed release scope without
  component-by-component fan-out in the UI

Closes:

- the remaining collection-scale context-management gap from the legacy system

## Execution order

1. `W80-bulk-governance-workbench`
2. `W81-source-driven-collections`
3. `W82-system-event-trace-and-operator-observability`
4. `W83-governance-decision-lifecycle`
5. `W84-collection-scoped-context-actions`

## Exit condition

This debt block is closed when:

- all five waves are `done`
- their operator-facing BDD and E2E coverage is green
- no remaining item from `docs/legacy-value-audit.md` still requires a new
  foundational capability rather than a refinement
