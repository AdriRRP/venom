# W58. Collection Active Findings

Wave: `W58-collection-active-findings`
Status: `done`
BDD impact: `extend`
Agentic impact: `none`
Infra profile: `none`

## Goal

Let operators query active findings for one closed release collection as one canonical scope, instead of reconstructing the release view manually from component and artifact lookups.

## Feature paths

- `features/view-active-findings.feature`

## Execution lanes

- `acceptance`
- `integration`
- `web`
- `e2e`

## Owned paths

- `crates/venom-domain/src/findings/**`
- `crates/venom-domain/src/inventory/**`
- `crates/venom-domain/examples/acceptance.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/lib/api.test.ts`
- `apps/web/src/routes/findings.tsx`
- `apps/web/src/routes/findings.test.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `docs/waves/W58-collection-active-findings.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W58-S01` | done | expose one canonical release-collection active findings view across domain, API, and UI | `scripts/check-wave.sh --wave W58-collection-active-findings` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep release scope closed and deterministic by reusing collection membership as the only source of query scope
