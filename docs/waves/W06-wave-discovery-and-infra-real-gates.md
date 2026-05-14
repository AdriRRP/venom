# W06. Wave Discovery And Infra Real Gates

Wave: `W06-wave-discovery-and-infra-real-gates`
Status: `done`
BDD impact: `none`
Agentic impact: `script`
Infra profile: `none`

## Goal

Make wave discovery deterministic from a compact product direction and formalize real-infrastructure rehearsal as a standard verification lane before product development begins.

## Feature paths

- `none`

## Execution lanes

- `unit`
- `integration`
- `infra`

## Owned paths

- `docs/product-direction.md`
- `docs/documentation-model.md`
- `docs/work-methodology.md`
- `docs/repo-structure.md`
- `docs/waves/WAVE-TEMPLATE.md`
- `scripts/rehearse-infra.sh`
- `scripts/check-slice.sh`
- `scripts/check-wave.sh`
- `README.md`
- `AGENTS.md`
- `CONTRIBUTING.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W06-S01` | done | add a compact canonical source for mission, outcomes, and wave discovery | inspect `docs/product-direction.md` and linked docs |
| `W06-S02` | done | add an explicit infra rehearsal lane and hook it into wave gates | `bash -n scripts/*.sh`, `./scripts/rehearse-infra.sh --profile full`, `./scripts/check-wave.sh --wave W06-wave-discovery-and-infra-real-gates --lane infra` |
| `W06-S03` | done | align onboarding and templates with the new model | inspect updated docs |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- the infra rehearsal lane currently skips cleanly because no local stack is wired yet
- once an infra compose file exists, the lack of `scripts/infra-smoke.sh` will fail explicitly
