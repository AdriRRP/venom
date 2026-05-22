Feature: Schedule collection scan
  Operators attach one periodic scan cadence to a closed release collection.

  Rule: A managed collection can own one durable periodic schedule
    Scenario: Reject scheduling for an unmanaged collection
      Given no managed components
      When VENOM schedules a deterministic collection scan for "release:2026.05" every 60 minutes due at unix ms 1000
      Then the collection scan schedule result is "rejected"

    Scenario: Plan one due collection scan without mutating the schedule state
      Given a managed component "component:payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      And VENOM creates collection "release:2026.05" named "May Release"
      And VENOM adds component "component:payments-api" to collection "release:2026.05"
      And VENOM schedules a deterministic collection scan for "release:2026.05" every 60 minutes due at unix ms 1000
      When VENOM materializes due collection scans at unix ms 1500 with limit 8
      Then the collection scan schedule result is "configured"
      And 1 due collection scans are materialized
      And the first due collection scan targets collection "release:2026.05"
      And the first due collection scan has 1 requests
      And collection "release:2026.05" next due is unix ms 1000

    Scenario: Durable replay preserves one collection scan schedule
      Given no managed components
      And a new durable state
      And VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably binds artifact "registry.example/payments@sha256:111" to component "component:payments-api"
      And VENOM durably creates collection "release:2026.05" named "May Release"
      And VENOM durably adds component "component:payments-api" to collection "release:2026.05"
      When VENOM durably schedules a deterministic collection scan for "release:2026.05" every 60 minutes due at unix ms 1000
      And VENOM reloads the durable state
      Then the collection scan schedule result is "configured"
      And collection "release:2026.05" next due is unix ms 1000
