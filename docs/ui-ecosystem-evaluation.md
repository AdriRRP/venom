# UI Ecosystem Evaluation

## Scope

This evaluation is for the first VENOM operator console.

It is not for a public marketing site.

The current backend already provides a real HTTP API in [apps/api/src/http/mod.rs](/Users/adrianramos/Cybersecurity/Venom/apps/api/src/http/mod.rs).

Current UI-relevant capabilities already exposed:

- component registration
- artifact binding
- provider runtime configuration
- scan request creation
- scan command status
- worker drain actions
- provider report ingestion
- active findings queries

## Selection criteria

The stack should fit these repo realities:

1. VENOM already has a Rust backend and durable domain core.
2. The first UI should be an operator console with forms, tables, filters, and explicit command status.
3. The default path should stay simple, deterministic, and easy to adopt by humans and agents.
4. The stack should not erode the platform goals of near embedded-grade efficiency and near nation-grade reliability.

Evaluation criteria:

- backend fit
- operator-console ergonomics
- runtime weight
- cognitive load
- ecosystem maturity for tables, filters, and server-state work
- risk of duplicating server responsibilities we already own in Rust

## Candidates

### 1. React + TypeScript + Vite + TanStack Router/Query/Table

Fit: `very strong`

Why it fits:

- Vite is explicitly optimized for a faster and leaner development experience and optimized production builds: [Vite guide](https://vite.dev/guide/)
- TanStack Query is purpose-built for fetching, caching, synchronizing, and updating server state: [TanStack Query overview](https://tanstack.com/query/latest/docs/framework/react/overview)
- TanStack Router gives type-safe routing and URL-state handling that fit operator filters well: [TanStack Router overview](https://tanstack.com/router/docs/docs)
- TanStack Table is a headless table/datagrid library with filtering, sorting, pagination, and virtualization primitives: [TanStack Table intro](https://tanstack.com/table/v7/docs/overview)

Pros:

- strongest ecosystem for dense operator UIs
- best fit for server-state heavy screens
- easiest path for active findings tables, filters, pagination, polling, and command-state views
- clean separation from the existing Rust API
- no need to duplicate backend behavior in a frontend meta-framework

Cons:

- more client-side JavaScript than a hypermedia-first approach
- React itself does not solve data-fetching, routing, or table concerns without extra libraries

Verdict:

- best default choice for VENOM if we want a durable operator console that will grow

### 2. htmx + Askama

Fit: `strong for a very thin console`

Why it fits:

- htmx keeps the model close to HTTP and HTML and typically expects HTML responses, not JSON: [htmx docs](https://htmx.org/docs/)
- htmx is dependency-free and does not require a build system: [htmx installation](https://htmx.org/docs/)
- Askama provides type-safe compiled templates in Rust: [Askama docs](https://docs.rs/askama/latest/askama/)

Pros:

- minimal client runtime
- very low conceptual overhead
- excellent fit for simple forms, server-rendered tables, and low-JS delivery
- aligns well with deterministic behavior and compact operational surfaces

Cons:

- weaker fit for richer client state, URL-driven filtering UX, or denser operator interactions
- tends to push UI composition back into server-rendered HTML endpoints
- would pull more presentation responsibility into the Rust app layer right when the backend is still growing

Verdict:

- best alternative if VENOM deliberately wants a hypermedia-first console with minimal JavaScript
- not my first choice for the likely medium-term complexity of this product

### 3. SvelteKit

Fit: `good, but not the best fit here`

Why it fits:

- SvelteKit targets robust, performant web applications: [SvelteKit introduction](https://svelte.dev/docs/kit/introduction)
- it provides routing, SSR/CSR/prerendering options, preloading, and Vite-based development: [SvelteKit introduction](https://svelte.dev/docs/kit/introduction)

Pros:

- lighter mental model than some React full-stack meta-frameworks
- good runtime characteristics
- solid fit for general web apps

Cons:

- less compelling than React for an operator console that likely wants best-in-class table and server-state tooling
- still introduces an app framework with server-side features we do not currently need because VENOM already has a Rust backend

Verdict:

- credible second-tier option
- not the strongest choice for this repo’s existing shape

### 4. TanStack Start

Fit: `promising, but wrong timing`

Why it fits:

- it offers full-document SSR, streaming, server functions, and Vite-based deployment: [TanStack Start overview](https://tanstack.com/start/latest/docs/framework/react/overview)
- it is currently documented as Release Candidate, feature-complete and API-stable, but not bug-free: [TanStack Start overview](https://tanstack.com/start/latest/docs/framework/react/overview)

Pros:

- attractive full-stack React ergonomics
- strong type-safety story

Cons:

- VENOM already has a Rust backend, so Start would duplicate server concerns
- RC status is not what I would choose for the first UI in a reliability-focused system

Verdict:

- worth revisiting only if VENOM later wants a JavaScript full-stack edge layer
- not the right default today

### 5. Leptos and Dioxus

Fit: `interesting, but not the right trade-off now`

Why they are attractive:

- Leptos offers full-stack Rust, progressive enhancement, and server functions: [Leptos home](https://leptos.dev/), [Leptos server functions](https://book.leptos.dev/server/25_server_functions.html)
- Dioxus offers fullstack, SSR, hydration, and Axum integration: [Dioxus fullstack docs](https://dioxuslabs.com/learn/0.6/guides/fullstack/)

Why I would not choose them now:

- their strongest value comes from collapsing frontend and backend into one Rust full-stack model
- VENOM has already invested in an explicit HTTP API and a clear backend boundary
- adopting them now would increase coupling and shift effort into a full-stack Rust UI model before the product’s operator workflows are stable

Verdict:

- technically appealing
- strategically premature for this repo

## Recommendation

Recommended default path:

- `apps/web`
- `React`
- `TypeScript`
- `Vite`
- `TanStack Query`
- `TanStack Router`
- `TanStack Table`

Why this is the best fit:

1. It respects the existing architecture instead of trying to replace it.
2. It gives the strongest ecosystem for operator-heavy views.
3. It keeps the UI as a thin consumer of the Rust API.
4. It lets VENOM validate UX without re-litigating backend ownership.

Recommended fallback path:

- `htmx + Askama`

Use that only if we explicitly decide that the operator console should stay mostly server-rendered and very low-JS for the foreseeable future.

## Recommended next wave

If we accept the default path above, the next practical wave should be:

- `W39-ui-operator-shell`

Scope:

- create `apps/web`
- wire a minimal operator shell
- add API health wiring
- add one first operational screen, likely active findings

Keep the first UI wave intentionally narrow:

- no auth redesign
- no large design system
- no SSR framework
- no frontend state machine complexity before the first real screen exists
