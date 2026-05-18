# W71. Governed findings filters

Wave: `W71-governed-findings-filters`
Status: `done`
BDD impact: `create`
Agentic impact: `none`
Infra profile: `db`

## Goal

Let operators query active findings by governance state over artifact-scoped and
release-scoped views so daily work can separate `open`, `risk-accepted`, and
`suppressed` findings without reconstructing the scope by hand.

## Feature paths

- `features/filter-governed-findings.feature`

## Execution lanes

- `unit`
- `integration`
- `acceptance`
- `e2e`

## Owned paths

- `crates/venom-domain/src/findings/**`
- `crates/venom-domain/examples/acceptance.rs`
- `apps/api/src/app/service.rs`
- `apps/api/src/http/mod.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/findings.tsx`
- `apps/web/src/routes/findings.test.tsx`
- `apps/web/e2e/operator-flow.spec.ts`
- `features/filter-governed-findings.feature`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W71-S01` | done | add governance-state filter to read-model queries | unit and acceptance checks |
| `W71-S02` | done | expose governance-state filtering through API and transport | integration checks |
| `W71-S03` | done | let operators filter governed findings from the console | web and e2e checks |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`
