@acceptance
Feature: Register component
  VENOM keeps an explicit inventory of the components it manages.

  Rule: The first registration puts a component under management
    Scenario: Register a new component
      Given no managed components
      When VENOM registers component "component:payments-api" named "Payments API"
      Then the component "component:payments-api" is under management
      And 1 component is under management
      And the registration result is "registered"

  Rule: Repeating the same registration is idempotent
    Scenario: Register the same component twice with the same canonical data
      Given a managed component "component:payments-api" named "Payments API"
      When VENOM registers component "component:payments-api" named "Payments API"
      Then the component "component:payments-api" is under management
      And 1 component is under management
      And the registration result is "unchanged"

  Rule: Conflicting re-registration is rejected
    Scenario: Register the same component key with a different name
      Given a managed component "component:payments-api" named "Payments API"
      When VENOM registers component "component:payments-api" named "Billing API"
      Then the component "component:payments-api" is under management
      And 1 component is under management
      And the registration result is "rejected"

  Rule: A managed component can own immutable artifacts
    Scenario: Bind an artifact to a managed component
      Given a managed component "component:payments-api" named "Payments API"
      When VENOM binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      Then the artifact "registry.example/payments@sha256:111" belongs to component "component:payments-api"
      And 1 artifact is bound to component "component:payments-api"
      And the artifact binding result is "bound"

    Scenario: Rebinding the same artifact is idempotent
      Given a managed component "component:payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      When VENOM binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      Then the artifact "registry.example/payments@sha256:111" belongs to component "component:payments-api"
      And 1 artifact is bound to component "component:payments-api"
      And the artifact binding result is "unchanged"
