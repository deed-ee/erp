# Services, persistence and operations

**Purpose and scope.** Use one ownership model for independently deployed Rust services and their MongoDB stores. Service profiles describe implementation shape, not new architecture.

## Mandatory rules

- Each service MUST own its invariants, data, migrations, credentials, grants and public contracts. Communicate through supported contracts, not foreign collections or service internals. Review existing libraries first; keep shared crates technical. Use `erp4us-messaging` for Rust NATS transport, not a second wrapper.
- Verify scope before use and preserve it in all relevant MongoDB selectors, including joins within the service, updates, deletes, deduplication keys and uniqueness indexes. Use scope-aware indexes for supported filters/sorts. Use bounded pages, stable ordering and revision checks where concurrent edits matter. Do not leak BSON persistence models as public DTOs.
- Use atomic updates or service-local transactions for invariants including writes plus outbox/inbox facts. Define retries, conflict behavior and an explicit migration/compatibility and rollback or restore plan. Test schema/index upgrades on representative data; describe retention and coordinated backup/restore of committed state, broker and object references where used. Unbounded queries and uncontrolled scans on request paths are prohibited.
- Provide meaningful structured operational signals, service/instance identity and version, stable error categories, correlation and trace IDs where available, and request/dependency timing where useful. Distinguish liveness, readiness, degraded or unavailable, and heartbeat presence. Do not treat missing samples as zero or a heartbeat as proof of readiness. Do not log credentials, tokens, business bodies or identifying payloads; redact before recording, not only at visualization.

## Recommended and prohibited patterns

*Small worker:* bootstrap config and identity, establish bounded dependency clients, handle one use case, own task shutdown and keep durable acknowledgment explicit. *Larger HTTP or messaging service:* use `main.rs` for bootstrap (configuration, logging, clients, signals, controlled drain), then domain, application use cases, ports when needed and HTTP/messaging/persistence adapters. Place typed input, error mapping and auth at boundaries. Prefer validated config to implicit production fallbacks; no mandatory module tree or boilerplate port traits. Use `tracing` JSON for new HTTP services unless an accepted decision says otherwise. Prefer allowlisted diagnostic fields and actionable, non-repetitive logs.

**Good example:** [Master Data bootstrap and scoped store](references.md#rust); [receipt delivery and restore tests](references.md#contracts-and-reliability). **Counterexample:** a service asks MongoDB directly for another owner's records, retries foreign writes without idempotency, or includes raw receipts in diagnostics.

**Enforcement.** Cargo topology and direct NATS dependency checks catch static boundary regressions. MongoDB credentials, query scopes, migration behavior, logs and runtime health require review and integration evidence.

**Exceptions and references.** Use the [exception process](README.md#rule-strength-and-exceptions). Apply repository-specific constraints from [ADR 0001](../adr/0001-independent-services.md), the [architecture baseline](../architecture/baseline.md), and [MongoDB indexing](https://www.mongodb.com/docs/manual/indexes/), [schema validation](https://www.mongodb.com/docs/manual/core/schema-validation/) and [transactions](https://www.mongodb.com/docs/manual/core/transactions/). Do not introduce a mandatory exporter or another datastore based on this standard.
