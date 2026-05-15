# Contract Checks

This directory owns technical compatibility checks for ports and adapters.

Rules:

- use this lane for adapter compatibility, not canonical business behavior
- keep canonical business executable specs under `features/**`
- do not shape domain vocabulary around provider payloads
- provider contracts should prefer one complete snapshot per immutable artifact over provider-specific delta events
- keep both deterministic fixture-based checks and live compatibility checks when a real provider is introduced
- keep fixture reports under `tests/contracts/**` and run them through the canonical contract runner
- keep one small real fixture corpus per provider so contract checks exercise real payload shape
- use the infra lane for live scanner execution once the real provider path is wired
