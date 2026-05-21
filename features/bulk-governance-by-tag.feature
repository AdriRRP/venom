Feature: Bulk governance by tag
  Operators work one reusable component-tag cohort without reconstructing scope
  by release or by visible page only.

  Rule: One tag-scoped open cohort supports explicit bulk governance actions

    Scenario: bulk accept risk across one tag-scoped cohort
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" and severity "high"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably registers component tag "tag:api" named "API"
      And VENOM durably assigns component "component:payments-api" to tag "tag:api"
      And VENOM durably records the provider scan report
      And VENOM durably accepts risk for open findings in tag "tag:api" with minimum severity "high" and reason "Accepted API cohort"
      And VENOM queries active findings for component "component:payments-api" and artifact "registry.example/payments@sha256:111" with governance state "risk-accepted", minimum severity "high", offset 0, and limit 10
      Then the active findings page total is 1
      And the first active finding governance state is "risk-accepted"

    Scenario: bulk suppress findings across one tag-scoped cohort
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0002" in package "zlib" version "1.2.13" and severity "high"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably registers component tag "tag:api" named "API"
      And VENOM durably assigns component "component:payments-api" to tag "tag:api"
      And VENOM durably records the provider scan report
      And VENOM durably suppresses open findings in tag "tag:api" with minimum severity "high" and reason "Hidden API cohort"
      And VENOM queries active findings for component "component:payments-api" and artifact "registry.example/payments@sha256:111" with governance state "suppressed", minimum severity "high", offset 0, and limit 10
      Then the active findings page total is 1
      And the first active finding governance state is "suppressed"
