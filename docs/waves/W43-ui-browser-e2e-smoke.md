# W43. UI Browser E2E Smoke

Wave: `W43-ui-browser-e2e-smoke`
Status: `done`
BDD impact: `none`
Agentic impact: `script`
Infra profile: `none`

## Goal

Add the first browser-driven executable smoke flow for the operator console, wired into repo gates and CI without introducing hidden backend behavior or paid dependencies.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `e2e`

## Owned paths

- `apps/web/e2e/**`
- `apps/web/package.json`
- `apps/web/package-lock.json`
- `apps/web/playwright.config.ts`
- `apps/web/tsconfig.json`
- `.github/workflows/tests.yml`
- `docs/work-methodology.md`
- `docs/waves/ACTIVE`
- `docs/waves/W43-ui-browser-e2e-smoke.md`
- `scripts/check-web-e2e.sh`
- `scripts/check-slice.sh`
- `scripts/check-wave.sh`
- `scripts/check-web.sh`
- `scripts/README.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W43-S01` | done | define and activate the wave | `./scripts/check-slice.sh --wave W43-ui-browser-e2e-smoke --slice W43-S01 --path docs/waves/ACTIVE --path docs/waves/W43-ui-browser-e2e-smoke.md` |
| `W43-S02` | done | wire a deterministic browser E2E runner and add the canonical operator smoke flow | `./scripts/check-slice.sh --wave W43-ui-browser-e2e-smoke --slice W43-S02 --lane e2e --path apps/web/package.json --path apps/web/package-lock.json --path apps/web/playwright.config.ts --path apps/web/tsconfig.json --path apps/web/vite.config.ts --path apps/web/e2e --path scripts/check-web-e2e.sh --path scripts/check-web.sh --path scripts/check-slice.sh --path scripts/check-wave.sh --path .github/workflows/tests.yml --path docs/work-methodology.md --path scripts/README.md` |
| `W43-S03` | done | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W43-ui-browser-e2e-smoke` |
| `W43-S04` | done | make the browser E2E runner portable across local macOS and Linux CI temp directories | `./scripts/check-wave.sh --wave W43-ui-browser-e2e-smoke` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep browser E2E narrowly focused on the operator-critical path
- prefer deterministic local backend state over shared or pre-existing state
- keep E2E separate from unit-level UI checks so failures stay attributable
- the repeated manual operator loop is now captured by `scripts/check-web-e2e.sh` and `apps/web/e2e/operator-flow.spec.ts`
