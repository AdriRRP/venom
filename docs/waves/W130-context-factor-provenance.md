# W130. Context Factor Provenance

Wave: `W130-context-factor-provenance`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Expose factor-level contextual provenance so operators can see not only the
effective contextual traits but also which scope supplied each one.

## Owned paths

- `crates/venom-domain/src/inventory/component_inventory.rs`
- `crates/venom-domain/src/findings/contextual_risk.rs`
- `crates/venom-domain/src/findings/mod.rs`
- `crates/venom-domain/src/inventory/mod.rs`
- `crates/venom-domain/src/lib.rs`
- `apps/api/src/app/service.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/findings.tsx`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W130-S01` | done | carry per-factor context provenance through effective-context merge, contextual projections, API payloads, and findings UI labels | `unit`, `integration`, `web` |

## Language impact

`add`

## Invariant impact

`I2`, `I11`
