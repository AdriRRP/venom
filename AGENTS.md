# VENOM Agent Guide

## Goal

Build VENOM through small, verifiable, provider-agnostic changes.

## Always-on rules

1. Keep changes small and reversible.
2. Prefer deterministic scripts and tests over repeated prose.
3. Read only the files needed for the task.
4. Treat BDD as the contract for observable behavior.
5. Do not couple the domain to Wiz or any paid provider.
6. Do not introduce silent drops, fake completion, or hidden retries.
7. Leave owned documentation current before considering work complete.
8. If you detect a stable repeated workflow, convert it into a script or skill instead of re-explaining it again later.
9. If you detect overlapping guidance, compact or unify it unless there is a strong reason not to.
10. Report by delta: describe only what changed, failed, or needs a decision.
11. Prefer gate outcomes over command transcripts unless a failing detail matters.
12. Use the repo as canonical memory; do not restate stable project state when it has not changed.

## Read order

1. `AGENTS.md`
2. `docs/waves/ACTIVE`
3. `docs/work-methodology.md`
4. only what the task needs:
   - `docs/product-direction.md` when choosing or shaping a new wave
   - `docs/ubiquitous-language.md` for domain work
   - `docs/architecture-invariants.md` for architectural work
   - `docs/bdd-feature-model.md` for `.feature` changes
   - `docs/repo-structure.md` for layout changes
   - `docs/ui-visual-language.md` for `apps/web` layout, styling, and interaction work
5. only the active wave doc, ADRs, runbooks, or legacy docs you actually need

## Delivery loop

1. Identify the active `wave`.
2. Identify the smallest useful `slice`.
3. Update only the required code, tests, and owned docs.
4. Run the smallest meaningful verification set.
5. Commit the slice.
6. Run the full wave gate before push.

Before closing a wave, explicitly check:

- glossary impact
- invariant impact
- BDD impact
- reusable workflow impact
- documentation compaction opportunity

## Communication discipline

- Keep progress updates short and delta-only.
- Do not repeat branch, PR, URL, or clean-tree state unless it changed or is blocking.
- Prefer `slice gate green` or `wave gate green` over replaying command output.
- Only surface raw failing details when they help diagnose or decide the next step.
- Default to one wave and one PR at a time unless stack depth is clearly worth the extra context cost.

## Documentation policy

- Everything outside `docs/` must be in English.
- Keep one canonical doc per concept.
- Keep always-loaded guidance short and stable.
- Do not leave process, agentic, or domain docs stale after changing the reality they describe.

## Canonical docs

- BDD structure: `docs/bdd-feature-model.md`
- Product direction: `docs/product-direction.md`
- Domain language: `docs/ubiquitous-language.md`
- Architecture rules: `docs/architecture-invariants.md`
- UI visual language: `docs/ui-visual-language.md`
- Delivery process: `docs/work-methodology.md`
- Active wave pointer: `docs/waves/ACTIVE`
- Repository layout: `docs/repo-structure.md`

## Avoid by default

- Large always-loaded manifests
- Duplicate rules across multiple docs
- Provider-specific workflows in the default path
- Paid external services in the default verification path
