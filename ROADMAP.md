# Roadmap

This is a priority order, not a release schedule. Run the smallest useful slice continuously and keep new services independently deployable.

The [requirements catalog](docs/requirements/README.md) separates commitments from release scope. The milestone names below are catalog targets, not dates.

| Priority | Deliverable | Exit evidence |
|---|---|---|
| Now (Foundation) | Keep the local Master Data article register/get slice working | Compose smoke reaches MongoDB; duplicate SKU is rejected; service restart preserves the article. The organization-scoped SKU and single name are bootstrap conventions, not universal product rules. |
| P0 (First public demo) | Deliver the read-only Article walkthrough and continuous deployment path | Helm and Argo CD reconcile a private development deployment from a scanned GHCR digest; promote the same validated deployment tuple to an isolated demo. Seed/reset and migration checks, denied writes and egress, readiness, observability and an operator runbook are required before approving public ingress. No writable local credential, full Angular shell or order workflow is required. |
| P0 | [Real identity and tenant scope](https://github.com/deed-ee/erp/issues/2) | OIDC token verification with a self-hostable provider, tenant/organization membership and per-operation policies. Negative tests across tenants, organizations, wrong audience and missing permission. Retire the demo credential outside isolated tests. |
| P0 | [Reliable event slice](https://github.com/deed-ee/erp/issues/4) | Compose replica set and JetStream with per-service credentials; create and persist an article and outbox atomically; independent consumer with inbox deduplication. Exercise broker outage, duplicate delivery, ordering, forced restart, quarantine/replay and trace propagation. |
| P1 | [Registry and Angular shell](https://github.com/deed-ee/erp/issues/3) | Operator-approved manifests, login and scope switching, article form/detail from metadata at runtime, English/German labels and keyboard-accessible Playwright scenario. |
| P1 (Order MVP) | Order confirmation to reservation | Start with one tenant, one company context and one warehouse in the happy path. Reference one partner identity with roles, and use an explicit unit and exact decimal quantity; no assumption that every product line requires reservation. Existing organization/tenant negative tests remain required at the identity gate. Verify successful and shortage variants without cross-database writes. |
| P1 | Procurement and saga history | Shortage creates a proposal; authorized approval produces a purchase order; Inventory posts the receipt, then reserves against the waiting order; show history and recoverable failures in Angular. Restart each participating service and replay duplicates. |
| P2 | Document lifecycle and external service | Tenant-scoped versioned uploads/downloads with checksums, short-lived URL and orphan cleanup; independent non-Rust example registers through the protocol. |
| P2 | Agent integration | Authorized agent discovers and uses an allowlisted MCP query and approved write using the same domain rules, with audit, dry run and no mandatory LLM. |
| P2 (Writable demo) | Temporary personal tenant after real authorization and the guided demo | Verify sandbox isolation, quotas, rate limits, expiry and retryable scoped cleanup. Keep seeded guided companies read-only. |
| Before production release | Versioned production artifacts and community readiness | [License and contribution terms decided](docs/community/licensing.md); validate migrations and backup/restore, publish verified images and versioned charts with deployment documentation and known limits. Obtain approval before releasing. |

The [deployment strategy](docs/architecture/deployment.md) defines promotion and public exposure gates. Update demo fixtures and walkthroughs with each relevant slice rather than deferring the demo to a release milestone.

**After MVP (Extension pilots):** Select narrow needs from the [reference scenarios](docs/requirements/reference-scenarios.md). Start with a typed attribute or translation pilot, or a bounded multi-output trace slice when a real scenario justifies it. Do not start a package runtime, generalized workflow engine, or every future domain in parallel.

**Later (Domain expansion):** Add multi-entity operations, richer production, quality, lifecycle records and passport exports in independently reviewable slices. Defer accounting, payroll, general-purpose workflow, WASM, marketplace and multi-region infrastructure until there is a concrete use case. Country-specific tax rules are extension work, not universal core logic. No regulatory compliance is claimed.
