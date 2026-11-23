# ADR 0001: Independently deployable bounded contexts

**Status:** accepted for the bootstrap.

Use a monorepo for contributor discoverability, not a shared runtime. Each bounded context owns one deployable application and its data namespace, contracts and migration process. Platform capabilities may share a boundary where they share an invariant (e.g. service and semantic registration); business contexts do not share collections. Do not add a service without an executable use case. Reject both a modular monolith and per-entity microservices. The cost is explicit failures and reconciliation at boundaries; exercise those first in the order reservation saga.

Related decisions: [ADR 0002: Messaging and transactions](0002-at-least-once.md) and [ADR 0003: Semantic contracts](0003-semantic-contracts.md). See the [stack evaluation](../architecture/technology.md).
