# 0001. Repository structure and workspace layout

## Status

`Accepted`

## Context

The legacy repository had a useful separation between runtime entrypoints and business code, but it also accumulated too many top-level concerns and a broad `shared` runtime crate that became a natural dumping ground.

Before starting implementation, VENOM needs a repo shape that is:

- easy to navigate
- aligned with Cargo conventions
- minimal in top-level clutter
- compatible with DDD without over-crating early

## Decision

Use:

- a virtual Cargo workspace at the repo root
- `apps/` for runtime entrypoints
- `crates/` for Rust libraries
- one bounded-context crate to start: `crates/venom-domain`
- `features/` for canonical executable specs
- `tests/contracts/` for port and adapter compatibility checks
- `scripts/` for deterministic automations
- `fixtures/` for small local reusable test data
- `infra/` for local infrastructure assets

Do not create a generic shared crate until stable real reuse appears.

## Consequences

- the initial structure stays small and understandable
- Cargo commands can be standardized at the workspace root
- DDD boundaries remain possible without forcing many crates too early
- shared abstractions must earn their existence through real repeated use
