# W135. Context Factor Identity Explainability

Wave: `W135-context-factor-identity-explainability`
Status: `done`
BDD impact: `none`
Agentic impact: `none`
Infra profile: `none`

## Goal

Expose the concrete identity behind each effective context factor so operators
can see not only the scope type but also which profile or tag supplied it.

## Owned paths

- `crates/venom-domain/src/inventory/component_inventory.rs`
- `crates/venom-domain/src/inventory/mod.rs`
- `crates/venom-domain/src/lib.rs`
- `crates/venom-domain/src/findings/contextual_risk.rs`
- `apps/api/src/app/service.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/findings.tsx`
- `docs/ubiquitous-language.md`

## Slices

| Slice | Status | Goal | Verification |
|---|---|---|---|
| `W135-S01` | done | add per-factor source identity through effective-context merge, API payloads, and findings UI labels | `unit`, `web` |

## Language impact

`change`

## Invariant impact

`I7`, `I11`
