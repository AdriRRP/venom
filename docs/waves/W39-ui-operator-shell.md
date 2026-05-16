# W39. UI Operator Shell

Wave: `W39-ui-operator-shell`
Status: `done`
BDD impact: `none`
Agentic impact: `script`
Infra profile: `none`

## Goal

Introduce the first VENOM operator console as a thin web app over the existing Rust API, with frontend delivery discipline and verification in place from the start.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`

## Owned paths

- `apps/web/**`
- `.github/workflows/tests.yml`
- `.github/workflows/quality.yml`
- `docs/work-methodology.md`
- `docs/repo-structure.md`
- `scripts/check-web.sh`
- `scripts/check-wave.sh`
- `scripts/check-slice.sh`
- `scripts/README.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W39-S01` | done | define the wave, frontend discipline, and verification contract | `./scripts/check-slice.sh --wave W39-ui-operator-shell --slice W39-S01 --path docs/waves/ACTIVE --path docs/waves/W39-ui-operator-shell.md` |
| `W39-S02` | done | scaffold `apps/web` with deterministic frontend checks and tests | `./scripts/check-slice.sh --wave W39-ui-operator-shell --slice W39-S02 --lane unit --path apps/web --path scripts/check-web.sh --path .github/workflows/tests.yml --path .github/workflows/quality.yml` |
| `W39-S03` | done | add the first operator shell with API health wiring | `./scripts/check-slice.sh --wave W39-ui-operator-shell --slice W39-S03 --lane unit --path apps/web/src` |
| `W39-S04` | done | add the first active findings screen over the existing API | `./scripts/check-slice.sh --wave W39-ui-operator-shell --slice W39-S04 --lane unit --path apps/web/src` |
| `W39-S05` | done | close the wave and run the full wave gate | `./scripts/check-wave.sh --wave W39-ui-operator-shell` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- keep the UI thin and operator-focused
- do not duplicate backend behavior in the frontend
- no SSR framework in the first UI wave
- frontend quality, type safety, and tests are part of the default path, not optional follow-up work
