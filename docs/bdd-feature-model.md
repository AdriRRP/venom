# BDD Feature Model

## Goal

Keep `.feature` files stable, readable, and reusable across waves.

## Core rule

Canonical `.feature` files are organized by capability, not by wave, story, aggregate, or service.

Waves change canonical features. They do not create duplicate feature files for the same behavior.

## File strategy

### Default

Use one file per capability:

`features/<capability>.feature`

Example:

`features/register-component.feature`

### Split only when needed

Use a small capability folder when one file becomes too dense:

`features/<capability>/rNN_<rule>.feature`

Example:

- `features/manage-findings/r01_report-finding.feature`
- `features/manage-findings/r02_withdraw-finding.feature`

## Split triggers

Split a capability file only when at least one is true:

1. It contains more than 3 distinct business rules.
2. It needs more than one meaningful `Background`.
3. It grows beyond about 7 to 9 scenarios.
4. The scenarios no longer share a clear business headline.
5. Different rules need different fixtures or execution levels.

## Wave interaction

Each wave has one `BDD impact`:

- `none`: no canonical business `.feature` changes in this wave
- `create`: add the first canonical feature for a new capability
- `extend`: add rules or scenarios to an existing feature
- `refine`: improve language or structure without changing behavior
- `split`: break one feature into a small capability folder

Default expectation:

- most waves are `extend`

## Execution lanes

Use separate lanes for separate concerns.

### Canonical acceptance BDD

Purpose:

- business-readable capability behavior

Location:

- `features/*.feature`
- `features/<capability>/**`

Exclusion:

- `features/e2e/**`

Default tag:

- `@acceptance`

### Full-stack system E2E

Purpose:

- critical end-to-end flows through real wiring

Location:

- `features/e2e/**`

Required tag:

- `@e2e`

### Provider contract checks

Purpose:

- verify that adapters satisfy a port contract

Location:

- `tests/contracts/**`

Rule:

- provider contracts are not canonical business `.feature` files
- they may use Gherkin if that genuinely helps, but they are owned as technical contract checks, not as business capability docs

## Structure inside a feature

- `Feature`: one capability
- `Rule`: one business rule
- `Scenario`: one example of one rule

Guidelines:

- keep scenarios around 3 to 5 steps
- keep `Background` at 4 lines or fewer
- prefer declarative language over UI or transport detail

## Assertion style

Prefer business-observable outcomes:

- a component exists
- a finding is active
- a classification changed
- inventory shows the new state

Avoid in canonical acceptance Gherkin:

- Rust enum names
- table names
- queue names
- internal projection columns unless the read model itself is the public contract

## Language rules

1. Use the canonical terms from `docs/ubiquitous-language.md`.
2. Do not reintroduce replaced legacy names.
3. Do not use provider-specific language in generic scenarios.
4. Keep steps domain-centered and unambiguous.

## Tags

Keep tags minimal.

Recommended:

- `@acceptance`
- `@e2e` only for heavier full-stack scenarios
- `@provider-contract` only for contract checks that intentionally use Gherkin

Optional:

- `@wXX` if the wave link materially helps navigation

Rule:

- workflow scripts must not depend on `@wXX` tags for correctness

## Step-definition rule

Organize step definitions by domain concept, not by feature file name.

Good examples:

- component steps
- finding steps
- classification steps
- inventory steps
