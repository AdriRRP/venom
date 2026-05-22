# W109 Contextual Risk Model Enrichment

## Goal

Replace the flat contextual-risk sum with deterministic postures that better
match operator meaning.

## Why

The previous model used all context traits, but only as one linear
`context_pressure` score. That made materially different workloads look too
similar, especially:

- public edge vs internal critical services
- hardened private services vs merely unspecified internal workloads

## Scope

- enrich `contextual_risk_level` with posture-based rules
- keep deterministic behavior and bounded explainability
- extend BDD and unit coverage for differentiated internal/public outcomes

## Out of scope

- probabilistic scoring
- external CVSS-like enrichments
- new context profile fields

## Verification

- targeted domain tests for contextual risk
- acceptance gate
- full wave gate

## Closure notes

Closed when contextual risk distinguishes at least:

- public critical workloads
- internal critical workloads
- hardened private workloads

without regressing existing contextual projections or deterministic replay.
