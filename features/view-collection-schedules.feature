Feature: View collection schedules
  Rule: Scheduled release collections stay ordered and explicit for operators
    Scenario: List scheduled collections by next due time before unscheduled collections
      Given a managed component "component:payments-api" named "Payments API"
      And a managed component "component:billing-api" named "Billing API"
      And VENOM creates collection "release:2026.06" named "June Release"
      And VENOM creates collection "release:2026.05" named "May Release"
      And VENOM creates collection "release:2026.07" named "July Release"
      And VENOM adds component "component:payments-api" to collection "release:2026.05"
      And VENOM adds component "component:billing-api" to collection "release:2026.06"
      And VENOM schedules a deterministic collection scan for "release:2026.06" every 120 minutes due at unix ms 2000
      And VENOM schedules a deterministic collection scan for "release:2026.05" every 60 minutes due at unix ms 1000
      When VENOM lists collection schedules at unix ms 1500
      Then 3 collection schedules are visible
      And 1 collection schedules are due now
      And the first collection schedule targets collection "release:2026.05"
      And the first collection schedule members are 1
      And the first collection schedule is due "true"
      And the second collection schedule targets collection "release:2026.06"
      And the second collection schedule is due "false"
      And the third collection schedule targets collection "release:2026.07"
      And the third collection schedule has no periodic cadence

    Scenario: Durable replay preserves scheduled collection ordering
      Given no managed components
      And a new durable state
      And VENOM durably registers component "component:payments-api" named "Payments API"
      And VENOM durably creates collection "release:2026.05" named "May Release"
      And VENOM durably adds component "component:payments-api" to collection "release:2026.05"
      And VENOM durably schedules a deterministic collection scan for "release:2026.05" every 60 minutes due at unix ms 1000
      When VENOM reloads the durable state
      And VENOM lists durable collection schedules at unix ms 1500
      Then 1 collection schedules are visible
      And 1 collection schedules are due now
      And the first collection schedule targets collection "release:2026.05"

    Scenario: Materialized schedules expose last run time and command count
      Given a managed component "component:payments-api" named "Payments API" with artifact "registry.example/payments@sha256:111"
      And VENOM creates collection "release:2026.05" named "May Release"
      And VENOM adds component "component:payments-api" to collection "release:2026.05"
      And VENOM schedules a deterministic collection scan for "release:2026.05" every 60 minutes due at unix ms 1000
      When VENOM materializes due collection scans at unix ms 1500 with limit 8
      And VENOM lists collection schedules at unix ms 1500
      Then 1 collection schedules are visible
      And 0 collection schedules are due now
      And the first collection schedule targets collection "release:2026.05"
      And the first collection schedule last ran at unix ms 1500
      And the first collection schedule last enqueued 1 commands
