# W15. Git Delivery Discipline

Wave: `W15-git-delivery-discipline`
Status: `done`
BDD impact: `none`
Agentic impact: `script`
Infra profile: `none`

## Goal

Enforce that completed waves are verified and closed only from clean committed git state, so local progress cannot drift away from origin unnoticed.

## Feature paths

- `none`

## Execution lanes

- `unit`

## Owned paths

- `docs/waves/W15-git-delivery-discipline.md`
- `docs/work-methodology.md`
- `scripts/check-git-discipline.sh`
- `scripts/check-wave.sh`
- `scripts/README.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W15-S01` | done | persist the previously uncommitted local W07-W14 backlog into git history before more development continues | `git status --short`, `git log --oneline --max-count=5` |
| `W15-S02` | done | fail the wave gate when the tree is dirty or the wave doc is not marked done | `./scripts/check-git-discipline.sh --mode wave --wave W15-git-delivery-discipline` |
| `W15-S03` | done | make the git-discipline guard executable so the wave gate can call it directly | `./scripts/check-git-discipline.sh --mode wave --wave W15-git-delivery-discipline` |

## Language impact

`none`

## Invariant impact

`none`

## ADR impact

`none`

## Notes

- this is a corrective workflow wave after discovering unpushed local work across multiple closed waves
