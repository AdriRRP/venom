Feature: Manage release collections
  Operators manage one closed release scope over explicit managed components.

  Rule: A collection is a closed explicit scope over managed components

    Scenario: create one release collection
      Given no managed components
      When VENOM creates collection "release:2026.05" named "May Release"
      Then the collection result is "created"
      And managed collections are 1

    Scenario: add one managed component to one collection
      Given a managed component "component:payments-api" named "Payments API"
      And VENOM creates collection "release:2026.05" named "May Release"
      When VENOM adds component "component:payments-api" to collection "release:2026.05"
      Then the collection membership result is "added"
      And collection "release:2026.05" contains component "component:payments-api"
      And collection "release:2026.05" has 1 members

    Scenario: reject one unmanaged component from one collection
      Given no managed components
      And VENOM creates collection "release:2026.05" named "May Release"
      When VENOM adds component "component:payments-api" to collection "release:2026.05"
      Then the collection membership result is "rejected"
      And collection "release:2026.05" has 0 members

    Scenario: durable replay preserves one closed collection
      Given a new durable state
      And VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably creates collection "release:2026.05" named "May Release"
      When VENOM durably adds component "component:payments-api" to collection "release:2026.05"
      And VENOM reloads the durable state
      Then the durable state manages collection "release:2026.05"
      And collection "release:2026.05" contains component "component:payments-api"

  Rule: A collection may derive membership from one declared source

    Scenario: materialize one replace source into one collection
      Given a managed component "component:payments-api" named "Payments API"
      And a managed component "component:billing-api" named "Billing API"
      And VENOM creates collection "release:2026.05" named "May Release"
      And VENOM adds component "component:billing-api" to collection "release:2026.05"
      When VENOM configures a replace component-list source for collection "release:2026.05" with components "component:payments-api"
      And VENOM materializes the source of collection "release:2026.05"
      Then the collection source result is "configured"
      And the collection source materialization result is "materialized"
      And collection "release:2026.05" has a replace source over components "component:payments-api"
      And collection "release:2026.05" contains component "component:payments-api"
      And collection "release:2026.05" has 1 members

    Scenario: materialize one reconcile source without removing existing members
      Given a managed component "component:payments-api" named "Payments API"
      And a managed component "component:billing-api" named "Billing API"
      And VENOM creates collection "release:2026.05" named "May Release"
      And VENOM adds component "component:billing-api" to collection "release:2026.05"
      When VENOM configures a reconcile component-list source for collection "release:2026.05" with components "component:payments-api"
      And VENOM materializes the source of collection "release:2026.05"
      Then the collection source result is "configured"
      And the collection source materialization result is "materialized"
      And collection "release:2026.05" contains component "component:payments-api"
      And collection "release:2026.05" contains component "component:billing-api"
      And collection "release:2026.05" has 2 members

    Scenario: reject one source that declares an unmanaged component
      Given no managed components
      And VENOM creates collection "release:2026.05" named "May Release"
      When VENOM configures a replace component-list source for collection "release:2026.05" with components "component:payments-api"
      Then the collection source result is "rejected"

    Scenario: durable replay preserves one declared collection source
      Given a new durable state
      And VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably creates collection "release:2026.05" named "May Release"
      When VENOM durably configures a replace component-list source for collection "release:2026.05" with components "component:payments-api"
      And VENOM durably materializes the source of collection "release:2026.05"
      And VENOM reloads the durable state
      Then the durable state manages collection "release:2026.05"
      And collection "release:2026.05" has a replace source over components "component:payments-api"
      And collection "release:2026.05" contains component "component:payments-api"
