Feature: Classify finding
  Rule: Context profiles deterministically raise contextual risk over raw severity
    Scenario: A critical execution context elevates one medium finding
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" and severity "medium"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably registers context profile "context:internet-prod" named "Internet Production" marked internet exposed, production, and mission critical
      And VENOM durably assigns context profile "context:internet-prod" to component "component:payments-api"
      And VENOM durably records the provider scan report
      And VENOM queries contextual active findings for component "component:payments-api" and artifact "registry.example/payments@sha256:111" with minimum severity "unknown", offset 0, and limit 10
      Then the first contextual active finding raw severity is "medium"
      And the first contextual active finding risk is "critical"
      And the first contextual active finding context profile is "context:internet-prod"

  Rule: Missing component context keeps the raw risk level
    Scenario: One high finding without context stays high
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" and severity "high"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably records the provider scan report
      And VENOM queries contextual active findings for component "component:payments-api" and artifact "registry.example/payments@sha256:111" with minimum severity "unknown", offset 0, and limit 10
      Then the first contextual active finding raw severity is "high"
      And the first contextual active finding risk is "high"
      And the first contextual active finding has no context profile

  Rule: Collection defaults can fill missing component context without overriding component specificity
    Scenario: One collection default and one component override merge field by field
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a provider scan report with vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" and severity "medium"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably creates collection "release:2026.05" named "May Release"
      And VENOM durably adds component "component:payments-api" to collection "release:2026.05"
      And VENOM durably registers context profile "context:corp-api-baseline" named "Corporate API Baseline" marked production
      And VENOM durably registers context profile "context:payments-edge" named "Payments Edge" marked internet exposed and mission critical
      And VENOM durably assigns context profile "context:corp-api-baseline" to collection "release:2026.05"
      And VENOM durably assigns context profile "context:payments-edge" to component "component:payments-api"
      And VENOM durably records the provider scan report
      And VENOM queries contextual active findings for collection "release:2026.05" with minimum severity "unknown", offset 0, and limit 10
      Then the first contextual active finding raw severity is "medium"
      And the first contextual active finding risk is "critical"
      And the first contextual active finding context profile is "context:payments-edge"
