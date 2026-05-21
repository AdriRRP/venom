@acceptance
Feature: Bulk accept risk
  VENOM can durably accept risk for one filtered open cohort inside a release
  collection without widening the write model or mutating suppressed findings by
  accident.

  Rule: One filtered open collection cohort can be risk accepted in bulk
    Scenario: Accept risk for all high-or-above open findings in one release
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with a critical vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" and a low vulnerability "CVE-2026-0002" in package "busybox" version "1.36.0"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably creates collection "release:2026.05" named "May Release"
      And VENOM durably adds component "component:payments-api" to collection "release:2026.05"
      And VENOM durably records the provider scan report
      And VENOM durably accepts risk for open findings in collection "release:2026.05" with minimum severity "high" and reason "Accepted for this release"
      And VENOM queries active findings for collection "release:2026.05" with governance state "risk-accepted", minimum severity "unknown", offset 0, and limit 10
      Then the scoped active findings page total is 1
      And the first scoped active finding vulnerability is "CVE-2026-0001"
      And the first scoped active finding governance state is "risk-accepted"
      And the first scoped active finding governance reason is "Accepted for this release"
      And VENOM queries collection governance overview for "release:2026.05" with governance state "open", minimum severity "unknown", offset 0, and limit 10
      And the collection health open findings is 1
      And the collection health risk accepted findings is 1

    Scenario: Bulk risk acceptance survives durable reload
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
      And VENOM durably accepts risk for open findings in collection "release:2026.05" with minimum severity "unknown" and reason "Accepted for this release" until unix ms 1760000000000
      And VENOM reloads the durable state
      And VENOM queries active findings for collection "release:2026.05" with governance state "risk-accepted", minimum severity "unknown", offset 0, and limit 10
      Then the scoped active findings page total is 1
      And the first scoped active finding governance state is "risk-accepted"
      And the first scoped active finding governance reason is "Accepted for this release"
