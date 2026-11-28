# Coding in erp4us

**Mission:** Build a self-hostable, agent-native business platform and ERP that people can extend with independently built services. Read the [architecture](docs/architecture/baseline.md), [ADRs](docs/adr/0001-independent-services.md), [roadmap](ROADMAP.md) and [threat model](docs/architecture/threat-model.md) before editing.

- Use deployable bounded contexts, not a modular monolith or an application per entity. Services own invariants, databases, indexes, migrations, permissions and contracts; use only published interfaces for cross-service work. Keep shared crates limited to genuinely shared technical concerns.
- Separate domain rules, application use cases, HTTP/messages and persistence as complexity warrants. In Rust, use explicit typed errors and transformations; avoid panics in request paths. In Angular, use strict TypeScript, Signals, declarative UI and translation keys; do not evaluate manifest code. Use English for project materials.
- Keep tenant/organization and actor explicit from verified credentials. Deny by default; do not trust a client-supplied scope or granted permission. Service and agent calls have the same enforcement as human calls. Never commit secrets, log sensitive payloads or introduce unsafe production defaults.
- Use exact decimals, versioned data and historical snapshots where applicable. Persist domain writes and outbox atomically for durable events; consumers own deduplication and recovery. No exactly-once assumptions or distributed DB transaction. Make corrections to ledger entries with compensating entries.
- Define language-neutral messaging contracts before code. Use [`erp4us-messaging`](crates/erp4us-messaging/README.md) under `crates/`, not a service-local NATS wrapper. Follow the [wire specification](docs/specifications/nats-messaging.md): protocol major first, schema major last, full versions in headers, explicit compatibility ranges and tenant authorization. Preserve existing tests and coordinate with ADRs and GitHub issues. Keep shared business models, speculative services and additional repositories out of scope without an explicit architecture decision.
- Prefer existing standards, contracts and conventions; use additive version evolution and document breaking changes/migration in an ADR or RFC. Metadata cannot grant permissions or bypass invariants. No speculative framework, unneeded infrastructure, or mandatory SaaS.
- Definition of done: focused working change, meaningful domain and integration tests (including negative authorization/scope and messaging replay tests when applicable), contract checks, updated docs, and a reviewable diff. Run relevant checks and state exactly what ran or was blocked. Never weaken tests or bypass checks to get CI to pass.
- Do not choose a license, make a release, change visibility or make consequential trust, branding or architecture decisions without approval. Follow [CONTRIBUTING.md](CONTRIBUTING.md) and the [RFC process](GOVERNANCE.md#rfc-process).

## Start with the user's decision

- Design around what someone needs to accomplish, not around CRUD screens, tables, or service boundaries. State the actor, the question or task, the desired outcome, and what would prevent it. Keep lists and tables available for comparison, bulk work and detailed evidence; do not make users reconcile several views to get the first useful answer.
- Lead with the answer, then show evidence and the next permitted step. For the [proposed build-blocker experience](https://github.com/erp4us-org/erp4us/issues/126), “Why can't I build this product?” should identify a concrete blocker such as 12 missing end plates, distinguish 20 needed from 8 actually reserved for the order, show the path through the affected assembly, and link to the authorized facts. Expected, received and allocated are different; missing or stale evidence is not proof of readiness.
- Design toward a [global workspace input](https://github.com/erp4us-org/erp4us/issues/85): one entry point to find records, ask domain questions, and start permitted workflows. Show the interpreted scope, filters or time range and ask when intent is ambiguous. Give the same useful answer through direct navigation, with a keyboard- and screen-reader-accessible path. Neither path may require AI, voice or a mandatory provider. A question or recommendation does not purchase, book, post or approve anything. Writes remain distinct, explicitly intended actions under the same owner-side rules for people and agents.
- Use diagrams and icons when they explain a relationship, quantity or decision, not just to decorate a result. A dependency path or a capacity view needs equivalent text, meaningful labels and a non-color way to identify its states.
- In the smallest workflow proof, show that the intended role can reach a correct, evidence-backed decision, recognize uncertainty or a denied step, and act without re-entering data. Check a counterexample that does not need the proposed objects, such as a service project with no warehouse. Label examples and prototypes as such; do not use them as capability claims.

In the Angular UI, pair each button's visible translated action label with a relevant icon. Hide decorative icons from assistive technology; retain native button and keyboard behavior.

## Hand off work

Before finishing or pausing a task, update the PR description with the outcome, checks run and any remaining blocker or next step. If there is no PR, use the existing task issue; for local-only work, give the handoff in the final response. Link to the diff, CI run or decision record instead of copying logs. Keep one handoff per task, not a separate diary per agent.

## Requirements engineering

Check the [catalog](docs/requirements/index.md), linked issues and open PRs before proposing a capability. Use stable IDs, a business example and a counterexample; separate a requirement, a policy option and a deployment decision. Follow the [review process](docs/requirements/guide.md), preserve earlier IDs and decision history, and link contracts, PRs, tests and evidence as work lands. Never infer approval from an issue or verification from an open PR. Validate changed records and regenerate the public index.

## Dependency changes

Follow the [monthly maintenance policy](https://github.com/erp4us-org/.github/blob/main/docs/dependency-maintenance.md). Keep alerts and grouped security updates enabled, scheduled version PRs and auto-merge disabled. Preserve lockfiles and pins; review advisories, licenses, migration and rollback, and require passing gates before manual merge. Expedite critical fixes.
