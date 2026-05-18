Feature: Suppress finding
  Rule: Operators can durably suppress one active finding without losing traceability
    Scenario: Suppress one active finding in one release-scoped view
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
      And VENOM durably suppresses vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" on component "component:payments-api" and artifact "registry.example/payments@sha256:111" with reason "Known upstream false alarm"
      And VENOM queries active findings for collection "release:2026.05" with minimum severity "unknown", offset 0, and limit 10
      Then the first scoped active finding governance state is "suppressed"
      And the first scoped active finding governance reason is "Known upstream false alarm"

    Scenario: Durable replay preserves one suppressed finding
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably records the provider scan report
      And VENOM durably suppresses vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" on component "component:payments-api" and artifact "registry.example/payments@sha256:111" with reason "Known upstream false alarm"
      And VENOM reloads the durable state
      And VENOM queries active findings for component "component:payments-api" and artifact "registry.example/payments@sha256:111" with minimum severity "unknown", offset 0, and limit 10
      Then the first active finding governance state is "suppressed"
      And the first active finding governance reason is "Known upstream false alarm"
