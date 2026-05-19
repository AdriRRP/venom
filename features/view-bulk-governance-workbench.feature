Feature: View bulk governance workbench
  Rule: Operators see one explicit open cohort summary before acting in bulk over one release
    Scenario: Query one filtered collection cohort for bulk governance
      Given a new durable state
      And VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "container-image" "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably registers collection "release:2026.05" named "May Release"
      And VENOM durably adds component "component:payments-api" to collection "release:2026.05"
      And VENOM durably registers context profile "context:internet-prod" named "Internet Production" marked internet exposed, production, and mission critical
      And VENOM durably assigns context profile "context:internet-prod" to component "component:payments-api"
      And VENOM durably records provider report "fixture-provider" for component "component:payments-api" and artifact "container-image" "registry.example/payments@sha256:111" with findings:
        | vulnerability_id | package_name | package_version | severity |
        | CVE-2026-0001    | openssl      | 3.0.0           | critical |
        | CVE-2026-0002    | busybox      | 1.36.1          | low      |
      And VENOM durably suppresses finding "CVE-2026-0002" for component "component:payments-api" and artifact "container-image" "registry.example/payments@sha256:111" package "busybox" version "1.36.1" with reason "Known upstream false alarm"
      When VENOM queries collection governance overview for "release:2026.05" with governance state "suppressed", minimum severity "low", offset 0, and limit 10
      Then the bulk governance cohort targets 1 finding
      And the bulk governance cohort shows 1 critical risk finding
      And the bulk governance cohort shows 0 high risk findings
