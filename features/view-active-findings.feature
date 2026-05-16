@acceptance
Feature: View active findings
  VENOM rebuilds operator-facing active findings from durable history.

  Rule: Durable history survives reload for active findings
    Scenario: Reload durable state after one active provider finding
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably records the provider scan report
      And VENOM reloads the durable state
      Then the durable state manages component "component:payments-api"
      And the durable state shows artifact "registry.example/payments@sha256:111" belongs to component "component:payments-api"
      And 1 active finding is projected for component "component:payments-api" and artifact "registry.example/payments@sha256:111"
      And vulnerability "CVE-2026-0001" is active for component "component:payments-api" and artifact "registry.example/payments@sha256:111"

  Rule: Reload keeps withdrawn findings inactive
    Scenario: Reload durable state after a withdrawal snapshot
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably records the provider scan report
      And an empty current provider scan report
      And VENOM durably records the provider scan report
      And VENOM reloads the durable state
      Then 0 active findings are projected for component "component:payments-api" and artifact "registry.example/payments@sha256:111"

  Rule: Active findings queries stay ordered and bounded
    Scenario: Query active findings with severity filter and bounded page size
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with a critical vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" and a low vulnerability "CVE-2026-0002" in package "busybox" version "1.36.0"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably records the provider scan report
      And VENOM queries active findings for component "component:payments-api" and artifact "registry.example/payments@sha256:111" with minimum severity "high", offset 0, and limit 1
      Then the active findings page total is 1
      And the active findings page returned count is 1
      And the active findings page limit is 1
      And the first active finding vulnerability is "CVE-2026-0001"
