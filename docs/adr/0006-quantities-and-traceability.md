# ADR 0006: Exact measured quantities and durable lineage

**Status:** Proposed.

**Context:** Discrete, continuous and project processes differ in definitions and actual outputs. Recalls and product records need evidence beyond the lifetime of a message.

**Decision:** Use exact decimals and explicit dimensioned units. Store original measured values, converted values when material to the transaction, and the conversion context and precision applied. Distinguish released definitions/plans from actual execution; a transformation links independent collections of inputs and outputs. Persist lineage evidence with the service that records the business fact. Compose authorized traversal and lifecycle projections from versioned APIs/events plus rebuild sources that outlast broker retention. Preserve corrections and provenance; do not make a view another authoritative store.

**Trade-off:** Reject floating-point-only quantities, fixed conversions for contextual readings, single-output execution and a broker-only trace log. Owners must specify archive/rebuild and correction procedures; projections can lag and must surface freshness and incomplete evidence. First orders need only an explicit unit and exact quantity, not a conversion engine or genealogy service.

**Open:** Decimal scale per use case, units standard mapping, projection granularity, privacy/retention conflicts and coordinated rebuild across service archives.

**Requirements:** [BD-007](../requirements/README.md#bd-007) through [BD-011](../requirements/README.md#bd-011), [BD-013](../requirements/README.md#bd-013).
