# Cross-industry architecture review

The Rust, Angular, MongoDB, NATS JetStream and S3-compatible baseline stays in place. The bootstrap slice and its draft contracts stay unchanged. New ADRs are proposed for maintainer review, not rollout authorization.

| Existing ADR | Assessment |
|---|---|
| [0001: Independent services](../adr/0001-independent-services.md) | Retain service ownership and deployment independence. Compose navigation and digital product views through documented interfaces rather than creating a shared business database. Package activation does not imply installing a service per capability. |
| [0002: At-least-once messaging](../adr/0002-at-least-once.md) | Retain local atomicity, outbox/inbox and idempotent subscribers. Event delivery and repair are not retention of authoritative trace evidence. |
| [0003: Semantic contracts](../adr/0003-semantic-contracts.md) | Retain OpenAPI and JSON Schema with declarative semantics. Future package and translation manifests must use vetted bundles and must not grant permissions or encode arbitrary executable rules. Current manifests remain unchanged. |

| Assumption to avoid | Evidence or risk | Gate before extending |
|---|---|---|
| Tenant equals company; every record has one organization owner | Demo auth binds one fixed tenant/organization; MongoDB selectors and SKU uniqueness use both. | Design explicit sharing and verify isolation across organizations and tenants before real business data or shared records. See [0005](../adr/0005-scope-and-identifiers.md). |
| Article SKU is universal identity; one name works everywhere | The draft Article input accepts one restricted SKU and one plain name. | Keep that contract; version or adapt with a mapping to stable ID, scheme and scope when extending. Agree source-locale migration before treating legacy names as translated. See [0005](../adr/0005-scope-and-identifiers.md) and [0007](../adr/0007-localization-and-provider-boundaries.md). |
| Every order item is physical and every partner is exclusively buyer or supplier | The roadmap leads with stock reservation and procurement; no order model exists yet. | Use shared partner identity and an explicit fulfillment requirement; service orders do not need inventory. See [0004](../adr/0004-capabilities-and-policy.md). |
| Quantities are integer pieces; structures have exactly one output | No quantity or production contract exists. These are risks in future designs, not current defects. | Use explicit units on the first order contract; exercise mixed measurements and multi-output lineage before committing to production API shapes. See [0006](../adr/0006-quantities-and-traceability.md). |
| Durable transport is the system of record | The messaging ADR covers delivery/replay, not permanent business evidence. | Specify service-owned records and reconstruction after broker expiry before claiming traceability. See [0006](../adr/0006-quantities-and-traceability.md). |
| A manifest can implement policy or authorize provider disclosure | The draft protocol exposes discovery, permission references and label keys only. | Keep enforcement at the owners. Review package compatibility/permissions and egress before activation. See [0004](../adr/0004-capabilities-and-policy.md) and [0007](../adr/0007-localization-and-provider-boundaries.md). |

The [domain model](business-domain.md) defines target responsibilities. The [roadmap](../../ROADMAP.md) keeps the next runnable scope at identity, reliability and one order flow; [reference scenarios](../requirements/reference-scenarios.md) serve as design checks rather than a mandate to build six implementations.
