Feature: Manage component tags
  Operators manage reusable transversal component cohorts without changing the
  semantics of closed release collections.

  Rule: A component tag groups managed components across releases

    Scenario: register one component tag
      Given no managed components
      When VENOM registers component tag "tag:api" named "API"
      Then the component tag result is "registered"
      And managed component tags are 1

    Scenario: durable replay preserves one component tag membership
      Given a new durable state
      And VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably registers component tag "tag:api" named "API"
      When VENOM durably assigns component "component:payments-api" to tag "tag:api"
      And VENOM reloads the durable state
      Then the durable state manages component tag "tag:api"
      And component tag "tag:api" contains component "component:payments-api"

  Rule: Tag overlays merge with collection defaults and component-specific overrides

    Scenario: tag overlay fills only the fields not defined by the component
      Given no managed components
      And a new durable state
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably creates collection "release:2026.05" named "May Release"
      And VENOM durably adds component "component:payments-api" to collection "release:2026.05"
      And VENOM durably registers component tag "tag:api" named "API"
      And VENOM durably assigns component "component:payments-api" to tag "tag:api"
      And VENOM durably registers context profile "context:release-default" named "Release Default" marked production
      And VENOM durably registers context profile "context:api-overlay" named "API Overlay" marked VPN restricted
      And VENOM durably registers context profile "context:public-edge" named "Public Edge" marked internet exposed and mission critical
      And VENOM durably assigns context profile "context:release-default" to collection "release:2026.05"
      And VENOM durably assigns context profile "context:api-overlay" to tag "tag:api"
      And VENOM durably assigns context profile "context:public-edge" to component "component:payments-api"
      And VENOM reloads the durable state
      Then the durable state shows component "component:payments-api" resolves context in collection "release:2026.05" as internet exposed, production, mission critical, and VPN restricted

    Scenario: reject one conflicting tag overlay on the same component
      Given no managed components
      And a new durable state
      When VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably registers component tag "tag:public-api" named "Public API"
      And VENOM durably registers component tag "tag:internal-api" named "Internal API"
      And VENOM durably assigns component "component:payments-api" to tag "tag:public-api"
      And VENOM durably assigns component "component:payments-api" to tag "tag:internal-api"
      And VENOM durably registers context profile "context:public-edge" named "Public Edge" marked internet exposed and mission critical
      And VENOM durably registers context profile "context:internal-overlay" named "Internal Overlay" marked production
      And VENOM durably assigns context profile "context:public-edge" to tag "tag:public-api"
      And VENOM durably assigns context profile "context:internal-overlay" to tag "tag:internal-api"
      And VENOM durably registers context profile "context:private-edge" named "Private Edge" marked internal
      And VENOM durably assigns context profile "context:private-edge" to tag "tag:internal-api"
      Then the tag context assignment result is "rejected"
