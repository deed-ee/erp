# Coding in erp4us

**Mission:** Build a self-hostable, agent-native business platform and ERP that people can extend with independently built services. Read the [architecture](docs/architecture/baseline.md), [ADRs](docs/adr/0001-independent-services.md), [roadmap](ROADMAP.md) and [threat model](docs/architecture/threat-model.md) before editing.

- Use deployable bounded contexts, not a modular monolith or an application per entity. Services own invariants, databases, indexes, migrations, permissions and contracts; use only published interfaces for cross-service work. Keep shared crates limited to genuinely shared technical concerns.
- Separate domain rules, application use cases, HTTP/messages and persistence as complexity warrants. In Rust, use explicit typed errors and transformations; avoid panics in request paths. In Angular, use strict TypeScript, Signals, declarative UI and translation keys; do not evaluate manifest code. Use English for project materials.
- Keep tenant/organization and actor explicit from verified credentials. Deny by default; do not trust a client-supplied scope or granted permission. Service and agent calls have the same enforcement as human calls. Never commit secrets, log sensitive payloads or introduce unsafe production defaults.
- Use exact decimals, versioned data and historical snapshots where applicable. Persist domain writes and outbox atomically for durable events; consumers own deduplication and recovery. No exactly-once assumptions or distributed DB transaction. Make corrections to ledger entries with compensating entries.
- Prefer existing standards, contracts and conventions; use additive version evolution and document breaking changes/migration in an ADR or RFC. Metadata cannot grant permissions or bypass invariants. No speculative framework, unneeded infrastructure, or mandatory SaaS.
- Definition of done: focused working change, meaningful domain and integration tests (including negative authorization/scope and messaging replay tests when applicable), contract checks, updated docs, and a reviewable diff. Run relevant checks and state exactly what ran or was blocked. Never weaken tests or bypass checks to get CI to pass.
- Do not choose a license, make a release, change visibility or make consequential trust, branding or architecture decisions without approval. Follow [CONTRIBUTING.md](CONTRIBUTING.md) and the [RFC process](GOVERNANCE.md#rfc-process).

In the Angular UI, pair each button's visible translated action label with a relevant icon. Hide decorative icons from assistive technology; retain native button and keyboard behavior.

## Hand off work

Before finishing or pausing a task, update the PR description with the outcome, checks run and any remaining blocker or next step. If there is no PR, use the existing task issue; for local-only work, give the handoff in the final response. Link to the diff, CI run or decision record instead of copying logs. Keep one handoff per task, not a separate diary per agent.
