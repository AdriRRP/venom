@acceptance
Feature: Accept risk
  VENOM keeps explicit risk-acceptance decisions durable and visible on active
  findings without confusing read views with the write model.

  Rule: One active finding can be risk accepted with explicit rationale
    Scenario: Accept risk for one release-scoped active finding
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably creates collection "release:2026.05" named "May Release"
      And VENOM durably adds component "component:payments-api" to collection "release:2026.05"
      And VENOM durably records the provider scan report
      And VENOM durably accepts risk for vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" on component "component:payments-api" and artifact "registry.example/payments@sha256:111" with reason "Compensating control in place"
      And VENOM queries active findings for collection "release:2026.05" with minimum severity "unknown", offset 0, and limit 10
      Then the scoped active findings page total is 1
      And the first scoped active finding vulnerability is "CVE-2026-0001"
      And the first scoped active finding governance state is "risk-accepted"
      And the first scoped active finding governance reason is "Compensating control in place"

  Rule: Risk acceptance survives durable reload
    Scenario: Reload durable state after one risk acceptance
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably records the provider scan report
      And VENOM durably accepts risk for vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" on component "component:payments-api" and artifact "registry.example/payments@sha256:111" with reason "Accepted until patch window" until unix ms 1760000000000
      And VENOM reloads the durable state
      And VENOM queries active findings for component "component:payments-api" and artifact "registry.example/payments@sha256:111" with minimum severity "unknown", offset 0, and limit 10
      Then the active findings page total is 1
      And the first active finding vulnerability is "CVE-2026-0001"
      And the first active finding governance state is "risk-accepted"
      And the first active finding governance reason is "Accepted until patch window"
      And the first active finding governance until unix ms is 1760000000000
