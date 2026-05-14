# Documentation Model

## Goal

Keep VENOM documentation:

- minimal;
- easy to navigate;
- cheap for agents to consume;
- reliable enough to drive implementation.

## Operating principles

1. One concept, one canonical document.
2. Keep reference docs short and table-driven.
3. Keep explanation docs rare.
4. Put steps in methods or runbooks, not in reference docs.
5. Update docs in the same wave that changes meaning.
6. Improve the system when repeated friction becomes visible.

## Document types

This follows a simple Diataxis split:

- `reference`: used while working
- `explanation`: used to understand why the system is shaped this way
- `how-to`: used to execute a procedure

## Canonical set

| Path | Type | Owns | Update when |
|---|---|---|---|
| `AGENTS.md` | manifest | always-on rules for agents | common rules change |
| `CONTRIBUTING.md` | entrypoint | human onboarding | onboarding changes |
| `docs/documentation-model.md` | explanation | this system | doc structure changes |
| `docs/product-direction.md` | reference | mission, outcomes, and capability map | product direction changes |
| `docs/repo-structure.md` | reference | repo layout and ownership by directory | repo layout changes |
| `docs/bdd-feature-model.md` | reference | `.feature` ownership and shape | BDD rules change |
| `docs/ubiquitous-language.md` | reference | domain vocabulary | domain meaning changes |
| `docs/architecture-invariants.md` | reference | current architectural rules | a rule changes |
| `docs/work-methodology.md` | how-to | waves, slices, gates | delivery process changes |
| `docs/agentic-development.md` | explanation | token-efficient agent workflow | agent workflow changes |
| `docs/waves/ACTIVE` | reference | active wave pointer | active wave changes |
| `docs/waves/WXX-<slug>.md` | reference | one wave plan and trace | per wave |
| `docs/waves/WAVE-TEMPLATE.md` | reference | wave doc structure | wave template changes |
| `docs/adr/NNNN-<slug>.md` | explanation | one significant decision | when needed |
| `docs/adr/ADR-TEMPLATE.md` | reference | ADR structure | ADR template changes |
| `docs/runbooks/<name>.md` | how-to | one operational procedure | when runtime ops exist |
| `docs/runbooks/RUNBOOK-TEMPLATE.md` | reference | runbook structure | runbook template changes |
| `features/**` | executable spec | capability behavior | capability behavior changes |
| `features/FEATURE-TEMPLATE.feature` | reference | canonical feature structure | feature template changes |
| `tests/contracts/**` | reference | port and adapter compatibility checks | contract behavior changes |
| `scripts/**` | how-to | deterministic project automations | script behavior changes |

## Ownership rules

- `docs/ubiquitous-language.md` is the only glossary.
- `docs/architecture-invariants.md` is the only invariant list.
- `docs/product-direction.md` is the only compact source of mission, outcomes, and capability map.
- `features/**` are organized by capability, not by wave.
- `docs/waves/**` link to features and impacts; they do not duplicate Gherkin, glossary, or ADR text.
- `docs/waves/ACTIVE` is the only active-wave pointer.
- reusable workflow knowledge belongs in scripts or skills once it is stable enough to justify reuse.
- workflow execution belongs in `scripts/**`; workflow guidance belongs in skills or docs.

## Update rules

### When the active wave changes

Update:

- `docs/waves/ACTIVE`

### When domain meaning changes

Update:

- `docs/ubiquitous-language.md`
- the active wave doc

### When mission, outcomes, or capability boundaries change

Update:

- `docs/product-direction.md`

### When an architectural rule changes

Update:

- `docs/architecture-invariants.md`
- the active wave doc
- an ADR if the choice is significant or costly to reverse

### When executable behavior changes

Update:

- canonical `.feature` files under `features/`
- the active wave doc

### When repeated workflow friction is discovered

Update one of:

- a script under `scripts/`
- a skill under `agents/skills/`
- `docs/agentic-development.md`
- `AGENTS.md`

Rule:

- do not keep paying the same token or cognitive cost if a stable reusable asset would remove it

## Wave document minimum

Each wave doc must contain only:

- `Wave: WXX-<slug>`
- `Status: planned | active | done`
- `Goal`
- `BDD impact: none | create | extend | refine | split`
- `Agentic impact: none | docs | script | skill | compact`
- `Infra profile: none | db | messaging | full`
- linked feature paths
- ordered slices
- `Language impact`
- `Invariant impact`
- `ADR impact`

## Diagram policy

Use only diagrams that answer recurring questions.

Default set:

1. one domain relationship diagram in the glossary
2. one context/container diagram when runtime architecture stabilizes
3. dynamic or deployment diagrams only when the flow is hard to explain in text

## Load policy

- `always-on`: `AGENTS.md`
- `frequent`: active wave pointer, glossary, invariants
- `on-demand`: BDD model, methodology, active wave doc, ADRs
- `on-demand when planning`: product direction
- `rare`: runbooks, legacy analysis

## Not required

- task handover files in the repo
- rolling agent memory logs
- duplicated vendor-specific manifests
- one glossary per wave
- one feature file per slice
- broad markdown catalogs
