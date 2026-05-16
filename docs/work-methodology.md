# Work Methodology

## Goal

Deliver VENOM through a workflow that is simple, verifiable, and easy for humans and agents to follow.

## Core loop

1. Choose a `wave`.
2. Confirm its BDD impact.
3. Split it into small `slices`.
4. Implement one slice.
5. Update owned docs if meaning changed.
6. Capture reusable workflow improvements if the slice reveals them.
7. Run the slice gate.
8. Commit the slice.
9. Repeat until the wave is complete.
10. Commit the final slice that closes the wave.
11. Mark the wave doc `done`.
12. Run the full wave gate on the clean committed tree.
13. Push immediately.

## Definitions

### Wave

A coherent delivery increment that closes with a full green gate and a push.

### Slice

The smallest safe vertical change that can stand on its own in one commit.

## Wave discovery

When `docs/waves/ACTIVE` is `NONE`, use `docs/product-direction.md` as the only wave-discovery source.

Apply its priority and sizing rules, then record the selected wave in `docs/waves/ACTIVE`.

## Identity contract

### Active wave

The active wave is stored in:

- `docs/waves/ACTIVE`

Contract:

- exactly one line
- value is either `NONE` or `WXX-<slug>`

Lifecycle:

- set it to `WXX-<slug>` when a wave becomes active
- set it back to `NONE` when the active wave is completed and no next wave is being started in the same change

### Wave id

Use:

- `WXX-<slug>`

Examples:

- `W01-foundation`
- `W02-register-component`

Rules:

- `WXX` is zero-padded
- `<slug>` is lowercase kebab-case
- the wave doc path is `docs/waves/WXX-<slug>.md`

### Slice id

Use:

- `WXX-SYY`

Examples:

- `W01-S01`
- `W01-S02`

Rules:

- slices belong to one wave
- `SYY` is zero-padded inside the wave

### Slice status

Use:

- `planned`
- `in_progress`
- `done`
- `blocked`

### Commit subject

Use:

- `<type>(<area>): [WXX-SYY] <summary>`

Example:

- `feat(domain): [W02-S03] add duplicate registration rule`

## Persistence discipline

- a slice is not complete until it is committed
- a wave is not complete until the wave gate passed on a clean committed tree and the commit set has been pushed
- a green local test run on an uncommitted dirty worktree does not count as a completed wave
- the default wave gate must fail if the worktree is dirty or if the wave doc is not marked `done`

## Non-negotiable rules

1. Every change must be small, testable, and reversible.
2. The domain leads; frameworks and providers are details.
3. No business path may rely on silent drop behavior.
4. BDD describes observable behavior, not internal mechanics.
5. The default path must work without paid external services.
6. Documentation and agentic assets must describe the current reality, not an aspiration.

## Test strategy

### L0. Unit

Fast domain and policy tests.

For the UI:

- component and hook tests live here
- typecheck and lint are part of the same default frontend quality path

### L1. Integration

Real Postgres, migrations, event store, outbox, projections, and workers when relevant.

### L2. Infra rehearsal

Real production-shaped infrastructure such as PostgreSQL and the message broker, started the way the app expects them.

Purpose:

- catch startup, networking, broker, migration, readiness, and multi-process faults early
- fail explicitly if another compose-backed rehearsal is already running in the same repo

Owned under:

- `infra/**`
- `scripts/rehearse-infra.sh`
- `scripts/infra-smoke.sh`

### L3. Acceptance BDD

Real API or binary with local fixtures and local infrastructure.

Owned under:

- `features/**`

### L4. System E2E

Real app and real wiring for critical flows only.

Owned under:

- `features/e2e/**`
- `apps/web/e2e/**` when browser-driven UI flows become necessary

### Contract checks

Technical compatibility checks for ports and adapters.

Owned under:

- `tests/contracts/**`

## Continuous-improvement rule

Every wave must explicitly assess:

1. Did domain meaning change?
2. Did an architectural rule change?
3. Did canonical executable behavior change?
4. Did we repeat a workflow enough to justify a script or skill?
5. Did we find overlapping guidance that should be compacted?

Use this threshold for "repeated enough":

- the same manual sequence appeared at least twice in one wave; or
- the same manual sequence has already appeared in a previous wave and appeared again now

Expected action:

- if the answer is yes and the improvement is stable and low-risk, make it in the same wave
- if it is not yet stable, record `Agentic impact: docs` or `Agentic impact: compact` in the wave doc and keep the guidance minimal until the pattern hardens

## Gates

### Slice gate

Run only what the slice needs:

- format
- lints
- affected unit tests
- affected integration tests
- affected BDD when behavior changed
- affected frontend check and build steps when the slice touches `apps/web/**`

Script interface:

```text
scripts/check-slice.sh --wave WXX-<slug> --slice WXX-SYY [--lane unit|integration|infra|acceptance|e2e|contract] [--path <repo-path>...]
```

Rules:

- exits `0` on pass, non-zero on failure
- prints a final line `RESULT: PASS` or `RESULT: FAIL`
- must not require network or paid services by default
- may read `docs/waves/ACTIVE` for validation, but must not depend on ambient git state
- must accept explicit selectors so it does not need to scrape free-form markdown
- must fail explicitly if the repo contains acceptance, e2e, or contract artifacts whose runner is not wired yet
- must run infra rehearsal when the slice touches db, messaging, startup, migrations, delivery guarantees, or multi-instance behavior

### Wave gate

Run the full required set:

- all unit tests
- all integration tests
- required infra rehearsal
- all BDD
- acceptance or E2E checks with real local infrastructure where relevant
- frontend quality, tests, and build when the wave touches `apps/web/**`

Selection rule:

- the script may use paths, manifests, or explicit config
- it must not rely on optional `@wXX` tags as the only way to discover scope

Script interface:

```text
scripts/check-wave.sh --wave WXX-<slug> [--lane unit|integration|infra|acceptance|e2e|contract|full]
```

Rules:

- exits `0` on pass, non-zero on failure
- prints a final line `RESULT: PASS` or `RESULT: FAIL`
- runs the full required wave verification set
- may read the wave doc for convenience, but explicit CLI inputs remain canonical
- must fail explicitly if executable specs or contract assets appear before their gate runner exists
- must include infra rehearsal in the default full wave path
- must fail if the worktree is dirty or the wave doc is not `done`

### Heavy gate

Reserve for major waves, release candidates, or scheduled CI:

- resilience
- restart and recovery
- backpressure
- throughput and latency regression detection
- longer-running stress or load checks

Script interface:

```text
scripts/check-heavy.sh --wave WXX-<slug> [--lane resilience|recovery|backpressure|stress]
```

## CI policy

GitHub Actions should mirror local verification through stable scripts whenever practical.

Required repository checks:

- `quality`
- `tests`
- `audit`

Enforcement:

- configure these workflow checks as required in GitHub rulesets or branch protection
- use `docs/runbooks/github-required-checks.md` as the canonical setup note

Advisory or scheduled checks:

- `unused-deps`
- `dependency-freshness`
- future stress, resilience, and infra rehearsal workflows

Default expectation:

- advisory checks should run on `schedule` or `workflow_dispatch` by default
- do not attach an advisory-only gate to normal PR flow unless it has explicitly graduated into the required path

Rule:

- do not add a PR-required check until it is stable, high-signal, and cheap enough to run on the default path

## Provider rule

The core works with a canonical finding model. Providers adapt into it.

Always keep:

- a provider port
- a local-first fixture provider
- shared provider contract tests

Never require external credentials in the default inner loop.

## Infra rule

Every wave must decide one `Infra profile`:

- `none`
- `db`
- `messaging`
- `full`

Use `none` only when the wave cannot plausibly be invalidated by real infrastructure behavior.

## Performance and reliability rule

When a wave touches a hot path or a durable business path, the wave must explicitly consider:

- allocation and copy behavior
- cross-process and cross-network chatter
- restart, retry, idempotency, and backpressure behavior

## Documentation rule

When a slice changes:

- domain meaning, update `docs/ubiquitous-language.md`
- architectural rules, update `docs/architecture-invariants.md`
- executable behavior, update canonical `.feature` files
- repeated stable workflow, update a script or skill
- overlapping guidance, compact the relevant docs

## Slice done

A slice is done when:

- it passes its gate
- its commit stands on its own
- its owned docs are current
- any stable repeated workflow it exposed has been captured or intentionally deferred in the wave doc
- it does not violate an invariant
