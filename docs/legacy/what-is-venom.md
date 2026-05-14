# VENOM Legacy: what it was and what it was trying to become

## Scope

This document summarizes what VENOM was by reading the legacy codebase at:

- `/Volumes/media/Backup Stratio/cyber-venom/cyber-venom`

The key conclusion is that VENOM was not just a vulnerability scanner. It was an internal platform for contextual vulnerability lifecycle management centered on software components, with a serious architectural base and a clear ambition to run in a distributed way.

## Executive summary

VENOM stands for `Vulnerability Evaluation, Notification, Orchestration and Management`.

Its purpose was to:

- register software components, mainly container images and packages;
- ingest vulnerability findings from external providers, with Wiz as the main backend and Grype as an optional one;
- separate the canonical vulnerability from the concrete finding on a component;
- recalculate severity according to the component's real execution context;
- support operational and governance decisions on each finding;
- expose an API and web UI for the full workflow;
- prepare a transition from an embedded local architecture to a more resilient distributed one.

In simple terms: VENOM wanted to become a vulnerability intelligence and operations layer, not just a CVE repository.

## The problem it was solving

The code reflects a clear underlying problem:

- an external scanner detects vulnerabilities, but that is not enough to prioritize;
- the same vulnerability does not have the same risk in every component;
- teams need an operational view by component, collection, and advisory;
- they also need governance: risk acceptance, suppression, false-positive handling, withdrawal when a finding is gone;
- at scale they need automation, traceability, idempotency, and recovery.

VENOM attacked that problem with an explicit domain model.

## The product's core idea

The most distinctive idea is this:

- `Vulnerability`: canonical advisory, independent from a component.
- `Finding`: a concrete observation of that vulnerability on a given component and artifact.
- `Component`: the software asset under management.
- `ExecutionContext`: the real execution environment of that component.

`ExecutionContext` is transformed into deterministic environmental CVSS overrides. VENOM tried to move from "this CVE is severe in the abstract" to "this finding on this component in this environment has this operational risk".

That makes the product much closer to a contextual prioritization and governance platform than to a simple findings store.
