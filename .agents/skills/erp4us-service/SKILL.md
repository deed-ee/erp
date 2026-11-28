---
name: erp4us-service
description: Add or extend an erp4us service, owner use case, bootstrap, health endpoint or operational diagnostics.
---

Required reading: [service profiles and boundaries](../../../docs/engineering/services.md), [architecture](../../../docs/architecture/baseline.md).

1. Identify the owning service, datastore, contract and actor permissions before changing code.
2. Choose the smallest relevant profile; use tested runtime and health patterns. Define deadline, shutdown and read-only diagnostic semantics.
3. Run Rust and applicable contracts and isolated owner integration checks from the [matrix](../../../docs/engineering/quality-gates.md).

Do not create a service per entity, access a foreign store, or move business rules into shared crates.

**Done:** ownership, world-facing contracts, error paths and durable effects are verified without cross-service coupling.
