@acceptance
Feature: Report finding
  VENOM derives finding lifecycle from canonical provider scan reports over immutable artifacts.

  Rule: Only managed components can ingest provider scan reports
    Scenario: Report for an unmanaged component
      Given no managed components
      And a component "payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM records the provider scan report
      Then the report is rejected as "unmanaged-component"
      And 0 components are under management

    Scenario: Report for an artifact not bound to the managed component
      Given a managed component "payments-api" named "Payments API"
      And a component "payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM records the provider scan report
      Then the report is rejected as "unmanaged-artifact"

  Rule: The first scan report discovers active findings
    Scenario: First report for an immutable artifact
      Given a managed component "payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM records the provider scan report
      Then 1 finding is newly discovered
      And 1 finding is active for the artifact

  Rule: Repeating the same scan report does not rediscover the same finding
    Scenario: Repeated report for the same immutable artifact
      Given a managed component "payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      And an artifact "registry.example/payments@sha256:111"
      And a recorded provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      And a current provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      When VENOM records the provider scan report
      Then 0 findings are newly discovered
      And 1 finding is repeated
      And 1 finding is active for the artifact

  Rule: Missing findings are withdrawn from the active set
    Scenario: Next report no longer includes a previously active finding
      Given a managed component "payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      And an artifact "registry.example/payments@sha256:111"
      And a recorded provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0"
      And an empty current provider scan report
      When VENOM records the provider scan report
      Then 1 finding is withdrawn
      And 0 findings are active for the artifact
