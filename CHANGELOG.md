# Changelog

## Unreleased

- Added a loopback Keycloak login, verified organization selection, and a persisted NovaMotion manufacturing catalog with a browser-based BOM and gross requirements view.
- Added idempotently seeded NovaMotion customers and suppliers plus a scoped, paginated partner directory with optional profiles.
- Added a platform-only application readiness view with real probes, and an explicit reset command for the isolated synthetic preview.
- Added scoped Sales drafts with catalog-backed customer and product snapshots and exact EUR totals. Drafts do not confirm an order or reserve inventory.
- Added explicit Sales confirmation with an atomic outbox and recovery-aware JetStream publication.
- Added replay-safe Inventory consumption and a linked reservation view with actual reserved and outstanding finished-goods quantities.
- Replaced the shared demo bearer token with verified OAuth access tokens and per-operation grants scoped to the tenant and organization.
- Added a local German/English Angular workspace for registering and retrieving articles, with labeled icon buttons and a keyboard-driven browser scenario.

- Established the architecture baseline, public roadmap, contribution process and licensing proposal.
- Drafted a language-neutral service manifest, article semantic descriptor and business event envelope.
- Added local Master Data article registration and lookup with MongoDB, Compose and CI checks.
