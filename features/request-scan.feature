@acceptance
Feature: Request scan
  VENOM creates canonical scan requests only for managed components and owned immutable artifacts.

  Rule: Only managed ownership can produce a scan request
    Scenario: Request a scan for an unmanaged component
      Given no managed components
      When VENOM plans a deterministic scan for component "component:payments-api" and artifact "registry.example/payments@sha256:111"
      Then the scan planning is rejected as "unmanaged-component"

    Scenario: Request a scan for an unbound artifact
      Given a managed component "component:payments-api" named "Payments API"
      When VENOM plans a deterministic scan for component "component:payments-api" and artifact "registry.example/payments@sha256:111"
      Then the scan planning is rejected as "unmanaged-artifact"

  Rule: Managed ownership produces canonical scan requests
    Scenario: Plan a deterministic scan for an owned artifact
      Given a managed component "component:payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      When VENOM plans a deterministic scan for component "component:payments-api" and artifact "registry.example/payments@sha256:111"
      Then the scan request targets component "component:payments-api"
      And the scan request targets artifact "registry.example/payments@sha256:111"
      And the scan request freshness is "deterministic"

    Scenario: Plan a live scan for an owned artifact
      Given a managed component "component:payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      When VENOM plans a live scan for component "component:payments-api" and artifact "registry.example/payments@sha256:111"
      Then the scan request targets component "component:payments-api"
      And the scan request targets artifact "registry.example/payments@sha256:111"
      And the scan request freshness is "live"

  Rule: Executing a planned scan applies the provider snapshot
    Scenario: Execute a deterministic scan and discover a finding
      Given a managed component "component:payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM plans a deterministic scan for component "component:payments-api" and artifact "registry.example/payments@sha256:111"
      And VENOM executes the planned scan
      Then the executed scan uses provider "fixture-provider"
      And 1 finding is reported by the provider snapshot
      And 1 finding is newly discovered
      And 1 finding is active for the artifact

    Scenario: Execute a planned scan when the provider is unavailable
      Given a managed component "component:payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      And the provider is temporarily unavailable
      When VENOM plans a deterministic scan for component "component:payments-api" and artifact "registry.example/payments@sha256:111"
      And VENOM executes the planned scan
      Then the scan execution is rejected as "provider-error"

  Rule: A durable runtime makes scan execution explicit and replayable
    Scenario: Run a queued deterministic scan to completion
      Given no managed components
      And a new durable state
      And a new durable scan runtime
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably plans a deterministic scan for component "component:payments-api" and artifact "registry.example/payments@sha256:111"
      And VENOM durably enqueues the planned scan
      And VENOM durably runs the next queued scan
      And VENOM reloads the durable state
      Then the durable runtime result is "completed"
      And the durable scan command status is "completed"
      And the durable runtime has 0 pending scan commands
      And 1 active finding is projected for component "component:payments-api" and artifact "registry.example/payments@sha256:111"

    Scenario: A queued scan fails explicitly when the provider is unavailable
      Given no managed components
      And a new durable state
      And a new durable scan runtime
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And the provider is temporarily unavailable
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably plans a deterministic scan for component "component:payments-api" and artifact "registry.example/payments@sha256:111"
      And VENOM durably enqueues the planned scan
      And VENOM durably runs the next queued scan
      Then the durable runtime result is "failed"
      And the durable scan command status is "failed"
      And the durable runtime has 0 pending scan commands
      And the durable runtime error is "provider-error"
