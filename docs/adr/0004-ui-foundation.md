# ADR 0004: One shell and design system for operator-approved modules

**Status:** Proposed. This is the local preview implementation; production registry and identity remain separate decisions.

Keep the Angular design system inside `ui/`. Use CSS custom properties, native controls, strict TypeScript, Signals, Reactive Forms, Lucide and the CDK for targeted accessibility primitives. Prefer native semantics over custom widgets. CSS owns presentation and responsive layout, Angular owns route/navigation and component state, and services own authorization and validation. Defer a separate design-system repository, Tailwind configuration and commercial grids until there is a concrete need.

The shell renders trusted, versioned navigation metadata and contextual panel registrations. The first implementation registers components and routes at build time. For runtime discovery, consume operator-approved metadata with schema validation, map its IDs to trusted routes, components, icons and translation keys, and reject unknown versions or targets. Service manifests describe services and contracts, not executable UI. The visibility checks are presentation only, never authorization. The future verified session supplies installed/enabled modules and effective grants; the current local adapter has a fixed scope.

For organization switching, confirm against any dirty or pending command state before changing context, validate the new membership server-side, abort stale reads, clear scope-keyed caches, and only then activate the authorized destination. A rejected or cancelled switch leaves the current context intact. The current demo intentionally has no switcher. Never store credentials or scope grants as layout preferences.

Use the same page frame for business collections and details. A list adapter must return bounded pages with service-owned typed filters and sorting. Entity writes use service validation and expected revision; conflicts preserve the user's edits until explicitly reconciled. The current contract supports registration and lookup only. Optional agents must dispatch the same authorized commands as a human; suggestions, reviewable previews and confirmed execution are separate states. They cannot silently approve changes and are never required for UI operation.

Keep the public website independent: vendor reviewed static data and brand assets from a pinned application commit, not application code or a runtime asset endpoint. The app establishes the visual sources of truth; website releases may intentionally lag.
