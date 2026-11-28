# TypeScript, Angular and web engineering

**Purpose and scope.** TypeScript rules apply to the Angular workspace and Astro site. Angular rules apply only to `ui/`; [ADR 0004](../adr/0004-ui-foundation.md) is proposed and describes the current local pattern, not a production approval. Read the current lockfile and runtime pin rather than assuming versions match across repositories.

## Mandatory rules

- Keep strict TypeScript and Angular `strictTemplates`. Use descriptive domain names, explicit public boundary types, direct imports and type-only imports when appropriate. Do not introduce unexplained `any`, unchecked assertions or non-null escapes. Decode `unknown` at I/O boundaries for new or changed contracts; a generic JSON return type does not validate runtime values.
- Keep API models aligned with the published contract and represent mutually exclusive states with discriminated unions. Avoid duplicate API types, barrel-induced cycles, mutation of derived state and stale results across identity or scope changes. Writes use the service's error, revision and retry semantics.
- Use standalone components, Signals for local/derived state, declarative templates, Reactive Forms for validated inputs, native controls and CDK where necessary, Lucide icons, and CSS custom properties from the existing system. Keep domain rules at the owner service, not in UI metadata, templates, or client-side permission checks. No second state or styling framework without a decision.
- Start with the actor and decision: answer, evidence, next permitted step. Global input and direct UI actions SHOULD converge on the same operation and reusable result renderer where available. No UI-dependent invariant, synthetic evidence presented as live, or implicit write from a query or suggestion. Clear stale reads on context change; distinguish loading, empty, denied, stale, failed and partial data when relevant.
- Use translated action labels and visible icons on Angular buttons; hide decorative icons from assistive technology. Use stable keys, semantic HTML, keyboard operation, visible focus and non-color status cues. Use locale-aware formatting with explicit currency/zone without losing authoritative decimal precision. Use existing tokens for light/dark and compact/comfortable modes; do not add arbitrary colors to feature components.

## Recommended and prohibited patterns

Keep local state local. Use computed state rather than copying values across stores, and RxJS where streaming/cancellation semantics are needed. Split components at reusable responsibilities, not a line limit. Review keyboard use, focus, narrow layouts, both locales, and contrast. On Astro, keep pages static, use source-linked facts and shipped assets, and validate both locale catalogs; do not import application code at runtime or require build credentials for PRs.

**Good example:** [Products, page frame and accessibility checks](references.md#web). **Counterexample:** a command bar writes directly to a different endpoint than the screen, an unvalidated JSON cast grants access, or a color-only success chip asserts completion after a timed-out command.

**Enforcement.** Angular build, ESLint, formatter checks and Playwright/axe exist in the [quality matrix](quality-gates.md). Accessibility and architectural intent still require human review. The website uses its existing Astro check, Prettier, catalog, built links and fixture pipeline.

**Exceptions and references.** Use the [exception process](README.md#rule-strength-and-exceptions). Adopt strict typing and targeted lint rules from [TypeScript](https://www.typescriptlang.org/docs/), [typescript-eslint](https://typescript-eslint.io/) and the [Angular style guide](https://angular.dev/style-guide); keep current file naming rather than bulk renames. See [Angular security](https://angular.dev/best-practices/security), [Signals](https://angular.dev/guide/signals), [Astro](https://docs.astro.build/), [HTML](https://html.spec.whatwg.org/), [CSS](https://www.w3.org/Style/CSS/) and [WCAG 2.2](https://www.w3.org/TR/WCAG22/). Use WCAG 2.2 AA as the review target rather than claiming automated certification.
