Feature: View release dashboard
  Scenario: Query one release dashboard over managed collections
    Given no managed components
    And a new durable state
    And a component "component:payments-api"
    And an artifact "registry.example/payments@sha256:111"
    And a provider scan report with a critical vulnerability "CVE-2026-0001" in package "openssl" version "3.0.0" and a low vulnerability "CVE-2026-0002" in package "busybox" version "1.36.1"
    When VENOM durably registers component "component:payments-api" named "Payments API"
    And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
    And VENOM durably registers context profile "context:internet-prod" named "Internet Production" marked internet exposed, production, and mission critical
    And VENOM durably assigns context profile "context:internet-prod" to component "component:payments-api"
    And VENOM durably creates collection "release:2026.05" named "May Release"
    And VENOM durably adds component "component:payments-api" to collection "release:2026.05"
    And VENOM durably schedules a deterministic collection scan for "release:2026.05" every 60 minutes due at unix ms 1000
    And VENOM durably records the provider scan report
    And VENOM durably suppresses vulnerability "CVE-2026-0002" in package "busybox" version "1.36.1" on component "component:payments-api" and artifact "registry.example/payments@sha256:111" with reason "Known upstream false alarm"
    And VENOM durably registers component "component:billing-api" named "Billing API"
    And VENOM durably binds artifact "registry.example/billing@sha256:222" to component "component:billing-api"
    And VENOM durably creates collection "release:2026.06" named "June Release"
    And VENOM durably adds component "component:billing-api" to collection "release:2026.06"
    And VENOM queries the release dashboard at unix ms 1500
    Then the release dashboard manages 2 collections
    And the release dashboard shows 1 scheduled collection
    And the release dashboard shows 1 collection due now
    And the release dashboard shows 2 active findings
    And the release dashboard shows 1 open finding
    And the release dashboard shows 1 suppressed finding
    And the release dashboard shows 1 critical risk finding
    And the release dashboard shows 1 high risk finding
    And the first dashboard collection is "release:2026.05"
    And the first dashboard collection is due "true"
    And the first dashboard collection health total is 2
