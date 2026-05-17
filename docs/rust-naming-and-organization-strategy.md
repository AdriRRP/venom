# Rust Naming and Organization Strategy

## Goal

Provide one compact, explicit strategy for renaming and reorganizing VENOM so that the codebase becomes more semantic and easier to navigate without adding abstraction layers, runtime cost, or naming churn without payoff.

## Source baseline

This strategy is derived from:

- The Rust Book:
  - `Packages, Crates, and Modules`
  - `Separating Modules into Different Files`
- Cargo Book:
  - `Manifest`
  - `Cargo Targets`
- Rust Style Guide:
  - `Other style advice`
- Rust API Guidelines:
  - `Naming`
  - `Conversions`
  - `Getters`
  - `Iterators`

Canonical links:

- <https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html>
- <https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html>
- <https://doc.rust-lang.org/cargo/reference/manifest.html>
- <https://doc.rust-lang.org/cargo/reference/cargo-targets.html>
- <https://doc.rust-lang.org/style-guide/advice.html>
- <https://rust-lang.github.io/api-guidelines/naming.html>

## Non-negotiable rules

1. Rename only when the new name increases semantic precision or removes ambiguity.
2. Keep module moves and naming changes behavior-preserving.
3. Do not add crates, traits, or layers just to “look more DDD”.
4. Prefer smaller public surfaces and clearer paths over wider re-export sets.
5. Optimize for local readability first, external API stability second, and novelty never.

## Rust naming rules to enforce

### Casing

- crates and modules: `snake_case`
- types and traits: `UpperCamelCase`
- functions and methods: `snake_case`
- constants: `SCREAMING_SNAKE_CASE`

### Constructor and conversion names

- plain constructors: `new`
- constructors with required detail: `with_*`
- cheap borrowed views: `as_*`
- expensive borrowed or owned conversions: `to_*`
- ownership-taking extraction: `into_*`
- wrapped value extraction: `into_inner`

### Getter names

- do not use `get_*` for ordinary getters
- prefer `field()` and `field_mut()`
- reserve `get`/`get_mut` for indexed or checked lookup semantics

### Iterator names

- `iter`, `iter_mut`, `into_iter`
- iterator type names should match the producing method

### Word order

- keep one stable word order per concept family
- prefer standard-library-shaped names where a close analogue exists
- prefer `ParseXError` over `XParseError`, `RunNextScanResult` over mixed orders

## Organization rules to enforce

### Workspace and package level

- keep package names short and semantically stable
- avoid `-rs` and `-rust`
- keep one package per stable boundary, not one package per concept

### Crate level

- keep `lib.rs` thin
- re-export only the stable public vocabulary
- keep internal implementation details behind capability modules

### Module level

- group by stable capability or boundary, not by incidental pattern
- prefer directories like `inventory/`, `findings/`, `scanning/`, `integration/`
- move code to a new module only when it lowers navigation cost

### App level

- keep `app/`, `http/`, and `infra/` separate
- do not let transport DTOs, persistence logic, and application orchestration drift into one file
- keep app names explicit when a generic term like `service` hides responsibility

## Rename heuristics

Use these questions before renaming:

1. Is the current name too broad for what the type actually owns?
2. Is the current name too narrow for what the type now owns?
3. Does the name leak mechanism instead of role?
4. Does the name force repeated explanation in code review or docs?
5. Would a newcomer predict the file path from the concept name?

Rename only when at least one answer is clearly `yes`.

## Current codebase assessment

### Strong

- package names are already short and valid: `venom-domain`, `venom-api`
- top-level module split is good: `findings`, `inventory`, `scanning`, `integration`
- crate and app roots are relatively thin

### Weak

- several names are role-generic rather than semantically specific:
  - `AppService`
  - `PostgresBackend`
  - `DurableState`
  - `ComponentInventory`
- some public re-exports are broader than the stable mental model really needs
- provider names mix role and mechanism in inconsistent word order:
  - `DockerSyftGrypeProvider`
  - `FixtureSyftGrypeProvider`
- some modules still encode implementation detail rather than bounded meaning:
  - `http_integration_publisher`
  - `durable_scan_runtime`

## Recommended refactor order

### Phase 1. Public vocabulary audit

Goal:

- decide the canonical nouns of the system before renaming files

Actions:

- review every public type re-exported from `venom-domain`
- keep only names that belong to the stable domain language
- demote low-level helpers from crate-root exports when possible

### Phase 2. Domain semantic renames

Goal:

- make domain types reflect bounded meaning precisely

High-value candidates:

- `ComponentInventory`
  - reassess because it now owns collections, provider runtime references, and schedules
- `DurableState`
  - reassess because it is not generic state; it is durable domain state for inventory/findings/integration
- `FindingIngestion`
  - reassess if the type is actually a stateful aggregate boundary rather than just an ingest operation façade

Rule:

- rename one concept family per wave, not the whole crate at once

### Phase 3. App and infra semantic renames

Goal:

- make application and persistence names reveal responsibility

High-value candidates:

- `AppService`
- `PostgresBackend`
- `http_integration_publisher`

Rule:

- prefer names that answer “what role does this play?” over “what pattern is this?”

### Phase 4. Public-path simplification

Goal:

- shorten and stabilize import paths after renames settle

Actions:

- reduce unnecessary root re-exports
- keep high-value domain nouns re-exported
- let implementation-specific names live lower in the tree

## Recommended next waves

1. `W53-domain-public-vocabulary-audit`
   - tighten crate-root exports and choose canonical domain nouns
2. `W54-domain-semantic-renames`
   - rename the first bounded family of domain types
3. `W55-app-and-infra-semantic-renames`
   - rename app/infra roles and modules
4. `W56-import-path-compaction`
   - simplify final public paths once the vocabulary is stable

## Decision

For VENOM, the best Rust-aligned strategy is:

- semantic renaming in bounded waves
- module grouping by capability
- smaller public surfaces
- standard-library-aligned method names
- zero behavioral change per rename wave

Do not attempt one giant rename pass.
