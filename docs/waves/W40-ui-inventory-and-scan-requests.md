# W40. UI Inventory And Scan Requests

Wave: `W40-ui-inventory-and-scan-requests`
Status: `active`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Extend the operator console so an operator can register a managed component, bind one artifact, configure its provider runtime, and enqueue a canonical scan request from the UI.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/web/**`
- `docs/waves/ACTIVE`
- `docs/waves/W40-ui-inventory-and-scan-requests.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W40-S01` | planned | define the wave and the operator flow boundary | `./scripts/check-slice.sh --wave W40-ui-inventory-and-scan-requests --slice W40-S01 --path docs/waves/ACTIVE --path docs/waves/W40-ui-inventory-and-scan-requests.md` |
| `W40-S02` | planned | add UI navigation and canonical web API mutations for operator actions | `./scripts/check-slice.sh --wave W40-ui-inventory-and-scan-requests --slice W40-S02 --lane unit --path apps/web/src` |
| `W40-S03` | planned | add the managed component and artifact binding flow to the UI | `./scripts/check-slice.sh --wave W40-ui-inventory-and-scan-requests --slice W40-S03 --lane unit --path apps/web/src` |
| `W40-S04` | planned | add provider configuration and scan request flow to the UI | `./scripts/check-slice.sh --wave W40-ui-inventory-and-scan-requests --slice W40-S04 --lane unit --path apps/web/src` |
| `W40-S05` | planned | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W40-ui-inventory-and-scan-requests` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep the UI thin and operator-facing
- prefer explicit mutation results over optimistic UI state
- do not duplicate domain validation in the frontend
