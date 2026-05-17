Feature: Request collection scan
  Operators request one canonical scan batch over a closed release collection.

  Rule: A managed release collection expands to canonical scan requests over owned artifacts

    Scenario: Request a scan batch for an unmanaged collection
      Given no managed components
      When VENOM plans a deterministic collection scan for "release:2026.05"
      Then the collection scan planning is rejected as "unmanaged-collection"

    Scenario: Request a deterministic scan batch for a closed release collection
      Given a managed component "component:payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      And a managed component "component:billing-api" named "Billing API" with artifact "registry.example/billing@sha256:222"
      And VENOM creates collection "release:2026.05" named "May Release"
      And VENOM adds component "component:billing-api" to collection "release:2026.05"
      And VENOM adds component "component:payments-api" to collection "release:2026.05"
      When VENOM plans a deterministic collection scan for "release:2026.05"
      Then the collection scan batch targets collection "release:2026.05"
      And the collection scan batch has 2 requests
      And the first collection scan request targets component "component:billing-api"
      And the first collection scan request targets artifact "registry.example/billing@sha256:222"
      And the second collection scan request targets component "component:payments-api"
      And the second collection scan request targets artifact "registry.example/payments@sha256:111"
