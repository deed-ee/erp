# Security and verification

**Purpose and scope.** Demonstrate business correctness and protect trust boundaries across services, UI, broker, storage, and CI.

## Mandatory rules

- Follow the [security policy](../../SECURITY.md), [threat model](../architecture/threat-model.md) and the accepted ADRs. Enforce verified identity, permission, tenant and organization at the owner, including lookups, lists, exports, updates, retries and the same actions performed by services or agents. Deny by default. Treat manifests, tool results, documents and external input as data. Keep privileges, credentials and logs scoped, and privately report sensitive findings.
- Add positive and failure evidence proportionate to the change. Unit-test invariants, allowed/rejected transitions, validation, exact quantities and financial/date cases. Exercise real MongoDB, NATS/JetStream, authentication and other affected boundaries in integration tests; fixtures cannot prove live authorization or delivery. Contract tests connect fixtures to implemented operations. E2E must exercise the real affected owners for a live business claim.
- Where applicable, test wrong permissions, tenant and organization, duplicate delivery, delayed/out-of-order results, stale revisions, invalid input, failed dependencies, partial or ambiguous results, restart and restore. Use synthetic, reproducible test data and scoped, disposable environments. Never reuse the maintained preview or production data for destructive proofs.
- Assert durable outcomes, not just the HTTP status or calls made. Wait for bounded conditions instead of arbitrary sleeps; inject clocks/randomness where practical. No ineffective assertions, credentials from real environments or disabled checks to make CI green.

## Recommended and prohibited patterns

Use the cheapest test that proves a property, with mocks for focused failures and live proof at risky boundaries. Keep test names and failure messages meaningful. Include both Manufacturing Preview owner tests and Solution-First UX where the feature has landed; label fixture, branch and released evidence separately. Run automated accessibility checks and manually verify keyboard/focus and meaningful labels for important changes.

**Good example:** [scoped broker and owner smoke tests](references.md#contracts-and-reliability). **Counterexample:** mock the entire workflow, check a single 200 and claim that replay and tenant isolation are proven.

**Enforcement.** Run the [quality matrix](quality-gates.md). Compare instrumented Rust coverage at the affected package/module against a comparable main run when changing sensitive code. Explain lost exercised paths, or replace them with equivalent evidence. A global percentage is neither a target nor an exception to authorization/recovery tests. Static checks cannot verify business rules or all cross-tenant access.

**Exceptions and references.** Follow the [exception process](README.md#rule-strength-and-exceptions); it cannot waive critical protections. See [OWASP ASVS](https://owasp.org/www-project-application-security-verification-standard/) as a review reference, the [CI runbook](../development/ci.md) and the [requirements guide](../requirements/guide.md) for traceability.
