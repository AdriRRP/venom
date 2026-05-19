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
