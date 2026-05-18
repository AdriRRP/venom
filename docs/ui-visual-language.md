# UI Visual Language

## Scope

This document defines the canonical visual language for the VENOM operator console.

It is not a marketing-site style guide.

It exists to keep the application:

- beautiful enough that operators want to use it every day
- simple enough that dense operational work stays legible
- consistent enough that new screens do not feel improvised
- efficient enough to fit the platform target of near embedded-grade execution

## Decision

VENOM should use one visual direction:

- `Operator Editorial`

Meaning:

- editorial clarity over ornamental complexity
- dense operational information with visible hierarchy
- warm neutral canvases with cool signal accents
- strong semantic color for severity and state
- restrained motion and effects

Default mode:

- `light`

Follow-on direction:

- support dark parity later through the same semantic token model

This is a deliberate improvement over legacy:

- keep its memorable personality
- reduce its decorative cyber-noise
- improve long-session readability
- make tables, filters, and operator decisions feel faster and calmer

## What Legacy Got Right

The legacy UI contributed several high-value ideas worth preserving:

1. It had a distinct personality instead of a generic enterprise look.
2. Severity and operational state were visually meaningful.
3. IDs, versions, and technical values benefited from monospaced treatment.
4. Panels and surfaces had enough atmosphere to feel crafted.
5. The product looked like a serious operator console rather than a CRUD admin.

The most reusable technical base from legacy was:

- semantic severity colors
- subtle grid and depth treatment
- clear panel separation
- high contrast data surfaces

## What To Improve

Legacy visual value should not be copied literally.

Improve it by:

1. reducing glow, noise, and decorative surface effects
2. preferring semantic tokens over route-local raw colors
3. making tables, filter bars, and summaries the primary visual system
4. using fewer accent hues with stronger semantic discipline
5. optimizing for long working sessions, not screenshot novelty

## State-Of-The-Art Signals

These references were the most useful for a modern VENOM operator console:

### Datadog

Most relevant lesson:

- repeatable UX patterns let users pivot across products without relearning core interactions

Why it matters for VENOM:

- findings, collections, schedules, decisions, and dashboards must share the same interaction grammar

Reference:

- [DRUIDS, the design system that powers Datadog](https://www.datadoghq.com/blog/engineering/druids-the-design-system-that-powers-datadog/)

### Linear

Most relevant lesson:

- dashboards are powerful when charts, tables, metrics, and filters coexist without clutter

Why it matters for VENOM:

- release collections should become an operational surface, not a static list

Reference:

- [Linear dashboards](https://linear.app/docs/dashboards)

### Primer

Most relevant lessons:

- use design tokens instead of raw values
- keep typography efficient and hierarchy-driven
- do not rely on color as the main emphasis channel

Why it matters for VENOM:

- data density must stay readable even as screens grow

References:

- [Primer color usage](https://primer.style/product/getting-started/foundations/color-usage)
- [Primer typography](https://primer.style/product/getting-started/foundations/typography/)

### Radix

Most relevant lessons:

- 12-step color scales are useful for dense product UI
- accessible primitives reduce accidental interaction regressions

Why it matters for VENOM:

- semantic states need both subtle and emphatic variants

References:

- [Radix Themes color](https://www.radix-ui.com/themes/docs/theme/color)
- [Radix Primitives accessibility](https://www.radix-ui.com/primitives/docs/overview/accessibility)

### Atlassian

Most relevant lessons:

- foundations, components, and patterns should all be explicit
- dynamic tables are first-class operational UI, not an afterthought

Why it matters for VENOM:

- the console should treat tables, filters, pagination, and status primitives as core building blocks

References:

- [Atlassian Design System](https://atlassian.design/design-system/)
- [Atlassian Dynamic Table](https://atlassian.design/components/dynamic-table)

## Visual Direction

### Overall feel

VENOM should feel like:

- a premium control room
- calm, exact, and fast
- technically serious without looking sterile

VENOM should not feel like:

- a glowing cyberpunk gimmick
- a flat bootstrap admin
- a dashboard collage with no dominant information hierarchy

### Color model

Use semantic tokens only.

Token families should be:

- neutrals
- accent
- severity
- governance state
- schedule state
- focus and selection

Rules:

1. One dominant accent family for interaction.
2. Severity colors are reserved for vulnerability meaning.
3. Governance colors are separate from severity colors.
4. Raw `hex`, `rgb`, or `oklch` values should live only in the token layer, not in route-specific CSS.

Recommended direction:

- warm neutral canvas
- cool blue-cyan accent
- amber only as secondary signal, not primary brand color

Meaning:

- neutral surfaces stay comfortable for long sessions
- cool accent keeps the product distinctive and technical
- severity colors stay visually available because the primary accent is not fighting them

### Typography

Use:

- `IBM Plex Sans` for interface text
- `IBM Plex Mono` or `JetBrains Mono` for technical identifiers

Rules:

1. Headings should be compact and strong, not oversized.
2. Body text should optimize for scanability over personality.
3. CVEs, component keys, artifact identities, versions, and command ids should use the mono face.
4. Use size and weight before color to create hierarchy.

### Layout grammar

Every operational screen should be composed from the same base anatomy:

1. shell
2. page header
3. summary strip
4. control band
5. primary data surface
6. detail or secondary diagnostics surface

This means:

- the first readable answer is near the top
- actions stay close to the data they affect
- filters are visible but not louder than results
- the main table or list is always obvious

### Surfaces

Surfaces should be:

- layered
- lightly elevated
- softly separated
- low-glare

Rules:

1. Prefer one subtle shadow family.
2. Prefer thin borders over heavy outlines.
3. Use background texture only as a low-opacity atmospheric layer.
4. Avoid repeated decorative effects inside every component.

### Tables

Tables are a core product surface.

Rules:

1. Sticky, visually distinct headers when useful.
2. Comfortable but compact row height.
3. Hover states must aid scanning, not distract.
4. Severity, state, and due-now badges must be visually crisp and semantically obvious.
5. Pagination and result windows must be easy to spot.
6. Filters must feel like part of the same data surface, not a separate mini-app.

### Forms and commands

Rules:

1. One primary action per section.
2. Secondary actions should look clearly secondary.
3. Success, failure, and pending states must be explicit.
4. Inline feedback should appear near the action that produced it.
5. Avoid modal-heavy flows when inline mutation panels are enough.

### Motion

Motion should be purposeful and fast.

Rules:

1. Prefer `120ms` to `180ms` transitions.
2. Prefer opacity, translate, and subtle shadow shifts.
3. Avoid bounce, overshoot, or decorative parallax.
4. Loading should favor skeletons or local pending affordances over spinner-only waiting.

### Accessibility

Rules:

1. WCAG AA contrast is the minimum.
2. No state may be communicated by color alone.
3. Keyboard navigation and focus rings are first-class.
4. Dense data views must remain readable under browser zoom.

## Implementation Rules

Until a dedicated token package exists, the canonical implementation layer is:

- `apps/web/src/styles.css`

Rules:

1. Put raw visual values in one token section.
2. Consume tokens through semantic classes and variables.
3. Prefer semantic names such as `panel`, `summary-strip`, `status-badge`, or `danger-muted`.
4. Avoid decorative names such as `arcane`, `cyber`, `magic`, or similar in the long-term app surface.

## Process Integration

Any wave touching `apps/web/**` must assess:

1. visual-language impact
2. browser-verification impact
3. token-discipline impact

Expected behavior:

- if a new color, spacing rule, or surface treatment is introduced, update this document in the same wave
- if a new screen appears, it should follow the shell/header/summary/control/data anatomy unless there is a strong reason not to
- if a route introduces one-off styling values that should be reusable, move them toward the token layer immediately

## Immediate Follow-On Waves

Recommended UI sequence after this definition:

1. `W67-apply-visual-language-foundations`
   Bring `apps/web/src/styles.css` in line with this token and surface model.

2. `W68-findings-screen-operator-restyle`
   Make `findings` the canonical dense operator surface.

3. `W69-operations-screen-command-restyle`
   Bring mutation-heavy workflows under the same visual grammar.

## Non-Goals

This visual language does not require:

- a large component library before the app needs it
- a separate design-vault repo
- visual experimentation outside the wave process
- brand-heavy marketing patterns inside the operator console
