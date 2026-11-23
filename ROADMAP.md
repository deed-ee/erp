# Roadmap

This is a priority order, not a release schedule. Run the smallest useful slice continuously and keep new services independently deployable.

The long-term goal is a platform for interoperable business applications, not just a fixed ERP suite. The first ERP process proves that independent services, semantic metadata, the browser UI and agent tooling can work together without a mandatory cloud or AI provider. The [architecture](docs/architecture/baseline.md) describes the broader system; this roadmap tracks the path to the first public preview.

Track work in [GitHub milestones](https://github.com/deed-ee/erp4us/milestones). Issues own acceptance criteria and live status; a milestone is a scope and dependency boundary, not a release date or approval to publish.

| Priority | Deliverable | Exit evidence |
|---|---|---|
| Now | Foundation and Master Data article register/get | Compose smoke reaches MongoDB; duplicate SKU is rejected; service restart preserves the article. Draft contract checks, governance and initial threat model are present. |
| P0 | [Real identity and tenant scope](https://github.com/deed-ee/erp4us/issues/2) | OIDC token verification with a self-hostable provider, tenant/organization membership and per-operation policies. Negative tests across tenants, organizations, wrong audience and missing permission. Retire the demo credential outside isolated tests. |
| P0 | [Reliable event slice](https://github.com/deed-ee/erp4us/issues/4) | Compose replica set and JetStream with per-service credentials; create and persist an article and outbox atomically; independent consumer with inbox deduplication. Exercise broker outage, duplicate delivery, ordering, forced restart, quarantine/replay and trace propagation. |
| P1 | [Registry and Angular shell](https://github.com/deed-ee/erp4us/issues/3) | Operator-approved manifests, login and scope switching, article form/detail from metadata at runtime, English/German labels and keyboard-accessible Playwright scenario. |
| P1 | [Order-to-procurement MVP](https://github.com/deed-ee/erp4us/issues/16) | Master Data, Sales, Inventory and Procurement complete the shortage-to-reservation flow, with history and recovery in Angular. Restart each service and replay duplicates without cross-database writes or repeated effects. |
| P2 | [Document lifecycle](https://github.com/deed-ee/erp4us/issues/17) and [external service](https://github.com/deed-ee/erp4us/issues/20) | Tenant-scoped versioned uploads/downloads with checksums, short-lived URLs and orphan cleanup; an independent non-Rust example registers through the protocol. |
| P2 | [Agent integration](https://github.com/deed-ee/erp4us/issues/19) | Authorized agent discovers and uses an allowlisted MCP query and approved write using the same domain rules, with audit, dry run and no mandatory LLM. |
| Before preview | [Operational readiness](https://github.com/deed-ee/erp4us/issues/18) | [License and contribution terms decided](docs/community/licensing.md). Validate migrations, backup/restore, sandbox and lifecycle controls; add supply-chain CI, verified builds, screenshots, demo and known limits. Obtain approval before releasing. |

Defer accounting, payroll, general-purpose workflow, WASM, marketplace and multi-region infrastructure until there is a concrete use case. Country-specific tax rules are extension work, not universal core logic. No regulatory compliance is claimed.
