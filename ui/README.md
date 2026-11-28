# Workspace UI

The [local quickstart](../README.md#try-the-local-angular-workspace) is the runnable component showcase. No public login or production API adapter ships with this local preview.

| Pattern | Example | Usage |
|---|---|---|
| Page frame | `/products` | Use `erp-page-heading` with translated title, summary and typed breadcrumb keys. Project toolbar links and buttons. |
| Buttons, controls and fields | `/products/new` | Use native buttons, selects and inputs. Use `.button` for action links, `.primary` for the one primary action, and give decorative Lucide icons `aria-hidden`. Pair inputs with labels, required state and error descriptions. |
| Forms and sections | `/products/new` | Use typed Reactive Forms, explicit fieldsets and legends. `erp-feedback` provides a text status; return focus to the invalid field and preserve form data on service errors. |
| Lookup, empty and error states | `/products` and `/products/:id` | Use `erp-section` and `erp-feedback` for individual lookups. The Products name form and global search reuse [`executeArticleLookup`](src/article-lookup.ts) and [`erp-article-lookup-result`](src/article-lookup-result.ts), including bounded SKU choices. Treat unsupported lists differently from real empty results. `ListQuery` and `ListPage` are extension contracts, not an implemented data grid. |
| Command input and answer | Header on every workspace route | Use the native search form and keyboard-accessible suggestion buttons. Render a direct answer before interpreted filters, scope and evidence. If an exact name has several matches, list the bounded SKU choices before the evidence; never auto-select a candidate. Keep business actions outside the read adapter. See the [command specification](../docs/specifications/command-interface.md). |
| Entity details | `/products/:id` | Use the page frame and an `erp-section` with a definition list for read-only properties. Keep revision separate from the business identifier. Do not invent lifecycle or tabs when no service data supports them. |
| Contextual panel | **View help** in the Products area | Register an ID, localized title and approved component; choose it through route metadata. Add permission requirements when the service/session interface exists. The panel stacks instead of overlaying small screens. |

## Design and locale

Canonical assets and tokens live in [`public/brand`](public/brand). Inter and JetBrains Mono (SIL Open Font License) are bundled locally for the initial Latin locales. Site CSS may use the system fallback. Use `--surface-elevated` for shell popovers, `--ink`/`--muted` on surfaces, `--on-accent` on a primary fill, and `--edge-strong` for control boundaries. Don't use the decorative `--edge` as the only control boundary. Semantic colors always accompany text or an accessible label. The shell reflows the search field to its own line at 760px so it remains usable on a narrow viewport; at 1080px the contextual panel stacks. Use logical properties and test long strings.

Keep new strings in `src/messages.ts`; English is the source and fallback and German must provide the same keys. For new business formats pass the locale to `Intl`, with explicit currency and time zone where relevant; money and quantities are exact strings on the wire. Share only public resources with optional localization tooling. No translation or AI provider is used at runtime.

Storybook can help validate interactive component variants independently once a second module needs shared complex controls. For now the runnable routes and Playwright baselines cover actual use without a second build pipeline. Defer record autocomplete, dates/decimal/currency widgets, tabs, dialogs, grids and timelines until they have an API-backed consumer. Prefer native controls or CDK primitives when they arrive.

## Visual tests

Playwright compares Linux Chromium baselines for the light workspace and desktop/mobile Products pages and runs axe checks on the same screens. Run in the pinned Node version. Review intentional visual changes with `npx playwright test tests/shell.spec.ts --update-snapshots` on Linux. Automated rules do not certify WCAG conformance.
