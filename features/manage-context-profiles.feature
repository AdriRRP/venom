Feature: Manage context profiles
  Rule: Operators can define one reusable execution-context profile
    Scenario: Register one context profile
      Given no managed components
      And a new durable state
      When VENOM durably registers context profile "context:internet-prod" named "Internet Production" marked internet exposed, production, and mission critical
      Then the durable state shows 1 managed context profile
      And the durable state shows context profile "context:internet-prod" is named "Internet Production"

  Rule: Managed components can attach one context profile durably
    Scenario: Assign one context profile to one managed component and reload
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably registers context profile "context:internet-prod" named "Internet Production" marked internet exposed, production, and mission critical
      And VENOM durably assigns context profile "context:internet-prod" to component "component:payments-api"
      And VENOM reloads the durable state
      Then the durable state shows component "component:payments-api" uses context profile "context:internet-prod"
      And the durable state shows context profile "context:internet-prod" is internet exposed, production, and mission critical

    Scenario: Assign one default context profile to one collection and reload
      Given no managed components
      And a new durable state
      And a component "component:payments-api"
      And a component "component:billing-api"
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably registers component "component:billing-api" named "Billing API"
      And VENOM durably creates collection "release:2026.05" named "May Release"
      And VENOM durably adds component "component:payments-api" to collection "release:2026.05"
      And VENOM durably adds component "component:billing-api" to collection "release:2026.05"
      And VENOM durably registers context profile "context:internet-prod" named "Internet Production" marked internet exposed, production, and mission critical
      And VENOM durably assigns context profile "context:internet-prod" to collection "release:2026.05"
      And VENOM reloads the durable state
      Then the durable state shows collection "release:2026.05" uses default context profile "context:internet-prod"
