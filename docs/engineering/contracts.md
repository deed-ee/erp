# Contracts and messaging

**Purpose and scope.** Define independently usable HTTP, semantic and NATS contracts for UI, services and authorized integrations, including MCP and agents. Do not duplicate the wire specification here.

## Mandatory rules

- Start with the [Open Business Service Protocol](../specifications/business-service-protocol.md) (`1.0-draft`), [`contracts/`](../../contracts/), and the [NATS wire specification](../specifications/nats-messaging.md). Keep capability, operation and permission IDs stable. Use owner-declared versions and relative, reviewed references. Manifests, descriptors and structured results are declarative evidence and never grant execution or permissions.
- Document field meaning, units, optionality versus nullability, validation, response/error mapping, and supported limits, filters and sort order. Use stable operation IDs and bounded, predictable responses. Use additive optional changes in a compatible version; do not add a required v1 request field, change semantics, or remove a supported operation. A breaking change needs a new major and consumer/migration plan. Keep historical fixtures; do not rewrite them to conceal a regression.
- Define language-neutral contracts before implementing NATS producers/consumers. Use the same subject and header conventions, version axes, authorized actor and scope, configured channel, correlation and causation as the wire specification. Validate bidirectionally consistent metadata, declared version bounds and payload at the boundary. Do not rely on broker metadata or a signed-in client alone as proof of business authority.
- Queries do not mutate; commands express intended actions; events describe facts. Bound Core reply time, stable errors and retry. For durable messages commit the business write and outbox together, publish with stable identity, confirm JetStream acknowledgment before advancing, and ack a consumer only after its local effect and inbox commit. Document duplicate/conflict behavior, per-key ordering or gap handling, bounded retries, quarantine with safe metadata and authorized replay/recovery; never claim exactly-once solely from the broker. Keep config, platform health/diagnostics and business channels distinct.

## Recommended and prohibited patterns

Prefer fixtures that an independent reader can run and test old reader/new writer and old writer/new reader where applicable. Distinguish structured result evidence and uncertainty from presentation, and query results from confirmable writes. Generate client artifacts only with a clear owner, reproducible command and consistency check; never edit generated artifacts. No second subject grammar, speculative SDK or breaking migration by default.

**Good example:** [Article v1 contract and the bounded compatibility fixture](references.md#contracts-and-reliability). **Counterexample:** a consumer retries a posting without stable keys and assumes a broker message ID prevents a second business effect.

**Enforcement.** Existing OpenAPI and JSON Schema validation, descriptor references, wire parsing tests, and Article-specific compatibility checks; no claim of a complete compatibility diff or runtime proof from schema validation. Modify fixtures and owner tests together.

**Exceptions and references.** Use the [exception process](README.md#rule-strength-and-exceptions) and [ADR 0002](../adr/0002-at-least-once.md), [0003](../adr/0003-semantic-contracts.md) and [0005](../adr/0005-versioned-nats-transport.md). Use [OpenAPI](https://spec.openapis.org/oas/), [JSON Schema](https://json-schema.org/) and [NATS documentation](https://docs.nats.io/) for their respective formats; erp4us owns business semantics and authorization.
