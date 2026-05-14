# Agentic Development

## Goal

Keep VENOM easy to develop with agents while minimizing token cost and process noise.

## Preferred shape

1. One short always-on manifest: `AGENTS.md`
2. One small reusable skill per repeated workflow family
3. Deterministic scripts for repeated checks and scaffolding
4. Docs loaded on demand, not by default

## Why

This fits both current guidance and the project's needs:

- short persistent context is easier to keep correct
- stable prefixes are cheaper to reuse and cache
- scripts reduce prompt repetition
- capability-based docs and tests age better than wave-specific prose

## Use this split

- permanent common rules -> `AGENTS.md`
- human entry -> `CONTRIBUTING.md`
- active delivery context -> `docs/waves/ACTIVE` and one wave doc
- process steps -> `docs/work-methodology.md`
- BDD ownership -> `docs/bdd-feature-model.md`
- domain vocabulary -> `docs/ubiquitous-language.md`
- architecture constraints -> `docs/architecture-invariants.md`
- repeated execution workflow -> skills or scripts

## Add something new only if

1. It is reused often.
2. It is stable enough to stay accurate.
3. It saves more tokens than it costs.
4. It does not duplicate an existing document.

## Mandatory improvement rule

Agents working in this repo are expected to improve the agentic system, not only consume it.

When a wave reveals:

- repeated command sequences
- repeated prompting patterns
- repeated setup or verification steps
- duplicated or overlapping guidance

the agent must do one of these before closing the wave:

1. capture the workflow as a script
2. capture the workflow as a skill
3. compact the overlapping docs
4. explicitly keep it manual because the pattern is still unstable

The default should be to remove repeated future token cost once the pattern is real.

Use this threshold for "pattern is real":

- it has already repeated twice; and
- the likely future shape is stable enough that automation will not thrash

## Script vs skill rule

Prefer:

- `script` when the workflow is deterministic and shellable
- `skill` when the workflow is mostly about read order, decision rules, or orchestrating several assets

Do not create a skill for something that should just be a script.
Do not create either one if the workflow is still changing every wave.

## Avoid

- large always-loaded manifests
- many overlapping skills
- repo-persisted agent memory logs
- provider-specific default workflows
- aspirational rules the team does not actually follow
- scripts or skills created before the pattern is stable

## Current project choice

For VENOM, the preferred low-token agent setup is:

- compact root manifest
- very small skill set
- script-first verification
- docs loaded on demand
- provider-neutral default paths
