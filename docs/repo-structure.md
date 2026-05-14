# Repository Structure

## Goal

Keep the repo easy to navigate, easy to automate, and hard to turn into a dumping ground.

## Lessons from legacy

Keep:

- `apps/` for runtime entrypoints
- a Cargo workspace root
- one place for scripts
- one place for executable specs

Do not keep:

- `contexts/shared` as a generic runtime dumping ground
- tracked large local data such as `sboms/` and `tmp/`
- vendor-specific agent directories
- separate design vaults that duplicate the canonical docs

## Target layout

```text
.
├── .github/
│   └── workflows/
├── apps/
│   └── api/
├── crates/
│   └── venom-vulnerability-management/
├── docs/
├── features/
│   └── e2e/
├── tests/
│   └── contracts/
├── scripts/
├── fixtures/
└── infra/
```

## Ownership

### `.github/workflows/`

Owns repository-enforced CI gates.

Rules:

- keep workflows thin and deterministic
- let workflows call repository scripts when practical
- keep required checks small and high-signal
- move heavier or noisier checks to scheduled or manual workflows

### `apps/`

Owns executable entrypoints only.

Rules:

- keep application wiring here
- keep domain logic out of here
- add `apps/web` or `apps/cli` only when they actually exist

### `crates/`

Owns Rust libraries.

Current rule:

- start with one bounded-context crate
- add another crate only when a second stable boundary or truly shared library is justified

This is the main correction over legacy:

- DDD boundaries still matter
- but the repo should not over-crate early

### `docs/`

Owns canonical human and agent documentation.

### `features/`

Owns canonical acceptance and E2E executable specifications.

### `tests/contracts/`

Owns port and adapter compatibility checks.

### `scripts/`

Owns deterministic automations.

### `fixtures/`

Owns local reusable test data and provider fixtures.

Rules:

- only small deterministic assets
- no large operational dumps

### `infra/`

Owns local infrastructure assets such as compose files, migrations, local environment support, and repository-host support assets that should stay out of always-loaded docs.

Rules:

- keep one compact local stack definition when possible
- use compose profiles to activate only the services needed for a rehearsal lane
- pair any real stack with one deterministic smoke or rehearsal script

## Rust workspace rules

Use a virtual workspace at the repo root.

Why:

- Cargo workspaces share one lockfile and target directory
- root commands can run across members
- `default-members` can keep root commands predictable

Use Cargo conventions inside each package:

- `src/lib.rs` for libraries
- `src/main.rs` for binaries
- `tests/` for integration tests

## Scaling rule

Do not add a new top-level directory unless:

1. it owns a distinct kind of artifact
2. that artifact has a stable lifecycle
3. reusing an existing directory would reduce clarity

## Decision

For the new VENOM repo:

- keep the top level small
- use `apps/` plus `crates/`
- start with one domain crate, not a premature shared crate
- keep tests, features, scripts, fixtures, and infra explicit
- treat large local data as untracked runtime material, not repository structure
