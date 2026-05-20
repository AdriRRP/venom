---
wave: W82-system-event-trace-and-operator-observability
created_at: 2026-05-20
status: done
owner: codex
---

# W82 System Event Trace And Operator Observability

## Goal

Expose command, scheduler, governance, and publication traceability as one
operator-facing event timeline.

## Slice plan

| Slice | Status | Goal |
| --- | --- | --- |
| W82-S01 | done | add one durable, rebuildable system-event projection in the domain |
| W82-S02 | done | expose the timeline through the API and Postgres store |
| W82-S03 | done | add one operator-facing UI route and browser smoke |

## Expected verification

- acceptance for observable timeline behavior
- integration for API and Postgres replay
- web and browser E2E for operator visibility
- full wave gate

## Impact check

- glossary impact: yes
- invariant impact: no
- BDD impact: yes
- reusable workflow impact: no
- documentation compaction opportunity: no
