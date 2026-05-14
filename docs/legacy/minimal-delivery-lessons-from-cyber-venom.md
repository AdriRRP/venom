# Minimal Delivery Lessons From cyber-venom Legacy

## Purpose

This document revisits the `cyber-venom` legacy repository with a narrow goal:

- extract the best already-applied practices around methodology, skills, and CI/CD;
- keep only what improves determinism, verification, and human readability;
- drop process layers that increase context cost and agent overhead.

Analyzed source:

- `/Volumes/media/Backup Stratio/cyber-venom/cyber-venom`

## Executive summary

The legacy repo contains a serious attempt at reproducible agentic delivery:

- a `Wave` / `Slice` model;
- strong local quality gates;
- reusable skills and workflows;
- infrastructure rehearsal scripts;
- operational runbooks and verification matrices;
- CI workflows aligned with local commands.

That part is valuable.

The overreach is equally clear:

- duplicated agent manifests (`AGENTS.md` + `CLAUDE.md`);
- a full `.agents/state/` persistence layer committed into the repo;
- mandatory handover and focus files for most non-trivial work;
- too many process documents that drift or bloat context;
- provider-specific workflows mixed into generic agent guidance.

The right move for the new VENOM is not to reject the legacy process model. It is to compress it aggressively.

## What the legacy got right

## 1. Wave / Slice as the core delivery unit

Legacy evidence:

- [`AGENTS.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/AGENTS.md>)
- [`CLAUDE.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/CLAUDE.md>)
- [`.agents/workflows/standard-task.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.agents/workflows/standard-task.md>)

This is the single best process idea worth preserving.

Why it works:

- it forces vertical progress;
- it reduces change scope;
- it maps naturally to BDD-driven delivery;
- it gives a stable abstraction for humans and agents.

Keep:

- one `wave` = one coherent capability;
- one `slice` = smallest safe vertical increment;
- one commit per slice;
- one full verification pass per wave.

Simplify:

- keep the model in one short methodology doc;
- do not spread it across multiple manifests and state files.

## 2. Local commands as the source of workflow truth

Legacy evidence:

- [`Makefile`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/Makefile>)
- [`.agents/tools/command-catalog.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.agents/tools/command-catalog.md>)

The legacy correctly centralized most workflows in Make targets:

- build;
- dev;
- unit tests;
- BDD;
- strict clippy;
- infra rehearsal;
- resilience rehearsal;
- security/dependency checks.

This is a strong pattern because it is:

- deterministic;
- shell-copyable;
- provider-neutral;
- cheap in tokens.

Keep:

- a small command surface with stable names;
- scripts behind commands when workflows are multi-step;
- local commands and CI using the same underlying entrypoints.

Simplify:

- keep command docs tiny;
- let the command names be the interface;
- do not maintain a separate long command catalog if `make help` already exists.

## 3. CI parity as a first-class principle

Legacy evidence:

- [`.github/workflows/clippy.yaml`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.github/workflows/clippy.yaml>)
- [`.github/workflows/tests.yaml`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.github/workflows/tests.yaml>)
- [`.github/workflows/ci-web.yaml`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.github/workflows/ci-web.yaml>)
- [`.github/workflows/audit.yaml`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.github/workflows/audit.yaml>)
- [`.github/workflows/udeps.yaml`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.github/workflows/udeps.yaml>)

The legacy was right to make CI explicit and strict.

Especially valuable:

- strict clippy parity;
- workspace-wide tests;
- separate web checks;
- explicit dependency and audit workflows;
- scheduled stress tests instead of putting all heavy checks in every push.

Keep:

- local and CI parity;
- separate workflows by concern;
- scheduled heavy reliability checks;
- explicit dependency/security automation.

Simplify:

- keep PR-required checks minimal and high-value;
- move noisy or expensive checks to scheduled runs or wave gates.

## 4. Real infrastructure rehearsals

Legacy evidence:

- [`scripts/infra-rehearsal.sh`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/scripts/infra-rehearsal.sh>)
- [`scripts/resilience-multi-instance.sh`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/scripts/resilience-multi-instance.sh>)
- [`scripts/capture-infra-run.sh`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/scripts/capture-infra-run.sh>)
- [`design/runbooks/w05-cutover-verification-matrix.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/design/runbooks/w05-cutover-verification-matrix.md>)

This is one of the strongest parts of the legacy process.

The scripts validate:

- API health;
- Postgres connectivity;
- RabbitMQ connectivity;
- multi-instance failover;
- evidence capture.

That is far more valuable than extra prose.

Keep:

- executable infra rehearsals;
- a small number of real E2E and failover checks;
- verification matrices only for high-risk cutovers.

Simplify:

- keep scripts;
- reduce the surrounding documentation to one short runbook per rehearsal;
- avoid sprawling operational markdown unless a procedure is truly expensive to rediscover.

## 5. Targeted skills instead of general prompt sprawl

Legacy evidence:

- [`.agents/skills/ci-quality-gates.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.agents/skills/ci-quality-gates.md>)
- [`.agents/skills/rust-ddd-cqrs-es.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.agents/skills/rust-ddd-cqrs-es.md>)
- [`.agents/skills/repo-architecture-alignment.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.agents/skills/repo-architecture-alignment.md>)

The good idea is not the directory itself. The good idea is:

- short, task-shaped reusable playbooks;
- tied to recurring work;
- grounded in real commands and repository truths.

Keep:

- skills only for repeated workflows;
- skill content that is short and operational;
- skill content that points to commands and files, not long explanations.

Simplify:

- start with one shared delivery skill;
- add a new skill only after repeated evidence;
- never duplicate the same rules in `AGENTS.md`, `CONTRIBUTING.md`, and multiple skills.

## What the legacy got wrong

## 1. Too much persistent repo state for the agent

Legacy evidence:

- [`.agents/state/current-focus.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.agents/state/current-focus.md>)
- [`.agents/state/decision-log.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.agents/state/decision-log.md>)
- [`.agents/state/handover.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.agents/state/handover.md>)

The problem is not that these files are useless. The problem is that they became:

- long-lived;
- cumulative;
- repetitive;
- expensive to parse;
- partially overlapping with commits, PRs, ADRs, and docs.

For the new repo, this is too much context cost for too little durable value.

Drop:

- mandatory committed task state files;
- repo-stored handover logs for normal work;
- rolling focus logs that accumulate across many waves.

Replace with:

- commits;
- BDD features;
- ADRs for expensive decisions;
- concise final summaries in PRs or task discussions.

## 2. Duplicate manifests and duplicated truth

Legacy evidence:

- [`AGENTS.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/AGENTS.md>)
- [`CLAUDE.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/CLAUDE.md>)

Both files contain overlapping process rules. The repo then adds `.agents/README.md`, workflows, tools, and state templates on top.

This creates:

- duplicate truth;
- drift risk;
- extra always-loaded context;
- more maintenance than value.

Drop:

- mirrored process manifests;
- vendor-specific duplication of the same project rules.

Keep:

- one vendor-neutral root manifest;
- one short contributing entrypoint.

## 3. Too much mandatory documentation review ceremony

Legacy evidence:

- `AGENTS.md` and `CLAUDE.md` require broad markdown consistency reviews for most non-trivial tasks.

The intention is good. The scope is too large.

Problem:

- reviewing all impacted markdown across agent docs, product docs, architecture docs, and design docs is expensive;
- it encourages ritual rather than precision;
- it increases token consumption for little gain on small slices.

Simplify:

- update docs only when behavior, workflow, or contracts change;
- review only the files truly affected;
- use one concise source of truth per topic.

## 4. Provider-specific workflows polluted the generic agent layer

Legacy evidence:

- [`.agents/workflows/wiz-production-integration.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.agents/workflows/wiz-production-integration.md>)
- [`.agents/skills/security-integration.md`](</Volumes/media/Backup Stratio/cyber-venom/cyber-venom/.agents/skills/security-integration.md>)

The security integration skill is useful in principle. The Wiz-specific workflow is not a good default repo primitive for the new VENOM.

Why:

- the new project must be provider-agnostic;
- paid-provider specifics should not shape the default development path;
- they increase context load for most tasks that do not need them.

Keep:

- provider-neutral integration guidance;
- contract-test mindset;
- secret-handling guardrails.

Drop:

- provider-specific workflow files from the default core repo process.

## 5. CI scope was too ambitious for the inner loop

Legacy evidence:

- `make check` includes dependency freshness, strict linting, type checks, and tests.
- dedicated workflows also run `cargo stale`, `cargo audit`, and `udeps`.

The principle is sound. The issue is gate placement.

`cargo stale` and similar checks are useful, but they create friction if treated as the default inner-loop gate for every small change.

For the new repo:

- keep them;
- do not make them the default slice gate.

## Keep / simplify / drop

## Keep

- Wave / Slice delivery model
- one commit per slice
- full verification per wave
- Make targets as workflow interface
- CI parity
- strict lint policy
- infra rehearsal scripts
- resilience / failover rehearsals
- ADRs for expensive decisions
- short reusable skills for repeated tasks

## Simplify

- `make check` into distinct slice and wave gates
- documentation updates into targeted scope only
- quality policy into one root manifest + one methodology doc
- skills into a very small set
- runbooks into short executable guides
- dependency/security checks into scheduled or wave-level gates

## Drop

- committed rolling agent state files
- duplicate `AGENTS.md` / `CLAUDE.md` truth
- provider-specific default workflows
- broad mandatory markdown review rituals
- large command catalogs if commands are already self-describing
- long handover documents as a normal workflow requirement

## Recommended minimal strategy for the new VENOM

## 1. Methodology

Keep only:

- `AGENTS.md`
- `CONTRIBUTING.md`
- `docs/work-methodology.md`

Execution model:

- one wave;
- several slices;
- one commit per slice;
- one push after wave verification.

## 2. Skills

Start with one project skill:

- `agents/skills/venom-delivery/SKILL.md`

Add new skills only when:

- the workflow repeats often;
- the workflow is stable;
- the workflow is expensive to restate.

## 3. CI/CD

### Slice gate

Fast and local:

- format;
- strict lint;
- affected unit tests;
- affected integration tests;
- affected BDD.

### Wave gate

Broader:

- full tests;
- full BDD;
- web checks if relevant;
- real-infra acceptance if relevant.

### Scheduled / manual heavy gates

- dependency freshness;
- cargo audit;
- unused dependencies;
- stress tests;
- resilience drills.

## 4. Scripts before prose

The legacy's best reusable artifacts were scripts, not state files.

Recommended future scripts:

- `scripts/check-slice.sh`
- `scripts/check-wave.sh`
- `scripts/new-wave.sh`
- `scripts/rehearse-infra.sh`
- `scripts/rehearse-resilience.sh`

## Final recommendation

The new VENOM should inherit the legacy's strongest delivery ideas:

- vertical slicing;
- real verification;
- local/CI parity;
- executable rehearsals.

But it should reject the legacy's process excess:

- too much persisted agent state;
- too much duplicated guidance;
- too much mandatory meta-documentation;
- too much provider-specific process in the generic workflow.

The right model is:

- fewer files;
- shorter manifests;
- more scripts;
- more executable checks;
- less repo-stored agent memory;
- one source of truth per concept.
