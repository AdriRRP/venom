Feature: Bulk suppress finding
  Rule: Operators can durably suppress one filtered open cohort inside one release-scoped view
    Scenario: Suppress one filtered open cohort in one release collection
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
      And VENOM durably suppresses open findings in collection "release:2026.05" with minimum severity "high" and reason "Known upstream false alarm"
      And VENOM queries active findings for collection "release:2026.05" with minimum severity "unknown", offset 0, and limit 10
      Then the first scoped active finding governance state is "suppressed"
      And the first scoped active finding governance reason is "Known upstream false alarm"
      And the second scoped active finding governance state is "open"

    Scenario: Durable replay preserves one bulk-suppressed collection cohort
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
      And VENOM durably suppresses open findings in collection "release:2026.05" with minimum severity "unknown" and reason "Known upstream false alarm"
      And VENOM reloads the durable state
      And VENOM queries active findings for collection "release:2026.05" with minimum severity "unknown", offset 0, and limit 10
      Then the first scoped active finding governance state is "suppressed"
      And the first scoped active finding governance reason is "Known upstream false alarm"
