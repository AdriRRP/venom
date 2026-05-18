Feature: View collection governance
  Rule: Release-scoped governance views keep one compact collection summary
    Scenario: Query one collection governance overview with a suppressed filter
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And an artifact "registry.example/payments@sha256:111"
      And a context profile "context:internet-prod" named "Internet Production" with internet exposure true, production true, and mission critical true
      And a provider scan report with a critical vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" and a low vulnerability "CVE-2026-0002" in package "busybox" version "1.36.1"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably creates collection "release:2026.05" named "May Release"
      And VENOM durably adds component "component:payments-api" to collection "release:2026.05"
      And VENOM durably registers context profile "context:internet-prod"
      And VENOM durably assigns context profile "context:internet-prod" to component "component:payments-api"
      And VENOM durably records the provider scan report
      And VENOM durably suppresses vulnerability "CVE-2026-0002" in package "busybox" version "1.36.1" on component "component:payments-api" and artifact "registry.example/payments@sha256:111" with reason "Known upstream false alarm"
      And VENOM queries collection governance overview for "release:2026.05" with governance state "suppressed", minimum severity "unknown", offset 0, and limit 10
      Then the collection health total active findings is 2
      And the collection health open findings is 1
      And the collection health suppressed findings is 1
      And the collection health risk accepted findings is 0
      And the collection health critical risk findings is 1
      And the collection health high risk findings is 0
      And the scoped active findings page total is 1
      And the first scoped active finding governance state is "suppressed"
