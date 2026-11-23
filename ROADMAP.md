# Roadmap

This is a priority order, not a release schedule. Run the smallest useful slice continuously and keep new services independently deployable.

| Priority | Deliverable | Exit evidence |
|---|---|---|
| Now | Foundation and Master Data article register/get | Compose smoke reaches MongoDB; duplicate SKU is rejected; service restart preserves the article. Draft contract checks, governance and initial threat model are present. |
| P0 | [Real identity and tenant scope](https://github.com/deed-ee/erp/issues/2) | OIDC token verification with a self-hostable provider, tenant/organization membership and per-operation policies. Negative tests across tenants, organizations, wrong audience and missing permission. Retire the demo credential outside isolated tests. |
| P0 | [Reliable event slice](https://github.com/deed-ee/erp/issues/4) | Compose replica set and JetStream with per-service credentials; create and persist an article and outbox atomically; independent consumer with inbox deduplication. Exercise broker outage, duplicate delivery, ordering, forced restart, quarantine/replay and trace propagation. |
| P1 | [Registry and Angular shell](https://github.com/deed-ee/erp/issues/3) | Operator-approved manifests, login and scope switching, article form/detail from metadata at runtime, English/German labels and keyboard-accessible Playwright scenario. |
| P1 | Order confirmation to reservation | Master Data partners, UoM and addresses; Sales quotation, conversion, snapshots and confirmation; Inventory warehouse, ledger, availability and reservation. Test both successful and shortage variants without cross-database writes. |
| P1 | Procurement and saga history | Shortage creates a proposal; authorized approval produces a purchase order; Inventory posts the receipt, then reserves against the waiting order; show history and recoverable failures in Angular. Restart each participating service and replay duplicates. |
| P2 | Document lifecycle and external service | Tenant-scoped versioned uploads/downloads with checksums, short-lived URL and orphan cleanup; independent non-Rust example registers through the protocol. |
| P2 | Agent integration | Authorized agent discovers and uses an allowlisted MCP query and approved write using the same domain rules, with audit, dry run and no mandatory LLM. |
| Before preview | Production and community readiness | [License and contribution terms decided](docs/community/licensing.md); validate migrations, backup/restore, sandbox and lifecycle controls; add supply-chain CI, verified builds, screenshots, demo and known limits. Obtain approval before releasing. |

Defer accounting, payroll, general-purpose workflow, WASM, marketplace and multi-region infrastructure until there is a concrete use case. Country-specific tax rules are extension work, not universal core logic. No regulatory compliance is claimed.
