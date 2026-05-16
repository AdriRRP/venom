# ADR 0004: Internal Module Organization

## Status

Accepted

## Context

VENOM has a healthy top-level repository layout, but the internal structure of `venom-domain` and `apps/api` has grown in a mostly flat way.

That is now creating three problems:

1. capability boundaries are harder to see than they should be;
2. large files are becoming informal coordination points;
3. structural drift is starting to hide hot paths, locks, and persistence seams that matter for the platform's near embedded-grade efficiency and near nation-grade reliability goals.

## Decision

Keep the repository top level as-is, but introduce explicit internal grouping by boundary and capability.

For `crates/venom-domain`:

- group code under capability-oriented module directories such as `inventory`, `findings`, `scanning`, and `integration`;
- keep `lib.rs` as a thin export surface rather than a growing flat index of unrelated modules;
- prefer selective re-exports over re-exporting almost everything by default.

For `apps/api`:

- separate HTTP routing and handlers from application orchestration and infrastructure adapters;
- group Postgres persistence and outbound integration publishers under explicit infrastructure modules;
- keep the router surface thin and make application/service hot paths easier to inspect independently of HTTP glue.

## Consequences

Good:

- clearer boundaries for humans and agents;
- lower risk of large-file coordination bottlenecks;
- easier review of locking, allocation, and durable write paths;
- easier future extraction of bounded modules without premature crate splitting.

Costs:

- some churn in imports and module paths;
- short-term refactor overhead with no immediate new business behavior.

## Non-goals

- no new framework or macro layer;
- no mandatory crate split in this wave;
- no extra abstraction unless it pays for itself in clarity or safety;
- no change in observable behavior.
