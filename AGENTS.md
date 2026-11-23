# Coding in erp4us

**Mission:** Build a self-hostable, agent-native business platform with ERP as its first application. People and agents use the same explicitly authorized business operations. Independent services integrate through the language-neutral service protocol, not through shared databases or changes to the platform core.

## Before starting work

1. Read the [roadmap](ROADMAP.md) for phase gates, scope and linked GitHub issues. Check the issue, dependencies and open PRs before starting; do not assume proposed or unmerged features exist.
2. Read the [architecture](docs/architecture/baseline.md), relevant [ADRs](docs/adr/0001-independent-services.md), [threat model](docs/architecture/threat-model.md) and affected [contracts](contracts/). For metadata, UI or integrations, read the [service protocol](docs/specifications/business-service-protocol.md).
3. Keep the issue as the source for task status and acceptance criteria. Record lasting decisions in an ADR or specification, not solely in issue comments. Keep the roadmap at milestone level.

## Working rules

- Use deployable bounded contexts, not a modular monolith or an application per entity. Services own invariants, databases, indexes, migrations, permissions and contracts; use only published interfaces for cross-service work. Keep shared crates limited to genuinely shared technical concerns.
- Separate domain rules, application use cases, HTTP/messages and persistence as complexity warrants. In Rust, use explicit typed errors and transformations; avoid panics in request paths. In Angular, use strict TypeScript, Signals, declarative UI and translation keys; do not evaluate manifest code. Use English for project materials.
- Keep tenant/organization and actor explicit from verified credentials. Deny by default; do not trust a client-supplied scope or granted permission. Service and agent calls have the same enforcement as human calls. Never commit secrets, log sensitive payloads or introduce unsafe production defaults.
- Use exact decimals, versioned data and historical snapshots where applicable. Persist domain writes and outbox atomically for durable events; consumers own deduplication and recovery. No exactly-once assumptions or distributed DB transaction. Make corrections to ledger entries with compensating entries.
- Prefer existing standards, contracts and conventions; use additive version evolution and document breaking changes/migration in an ADR or RFC. Metadata cannot grant permissions or bypass invariants. No speculative framework, unneeded infrastructure, or mandatory SaaS.
- Prefer a working vertical slice over empty service skeletons. Preserve the supported local Compose path and the ability to run without an LLM. Add executable contracts and synthetic examples when introducing cross-service integration.
- Definition of done: focused working change, meaningful domain and integration tests (including negative authorization/scope and messaging replay tests when applicable), contract checks, updated docs, and a reviewable diff. Run relevant checks and state exactly what ran or was blocked. Never weaken tests or bypass checks to get CI to pass.
- Do not choose a license, make a release, change visibility or make consequential trust, branding or architecture decisions without approval. Follow [CONTRIBUTING.md](CONTRIBUTING.md) and the [RFC process](GOVERNANCE.md#rfc-process).
