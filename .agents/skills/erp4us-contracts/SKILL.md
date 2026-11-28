---
name: erp4us-contracts
description: Design or review OpenAPI, JSON Schema, semantic descriptors, manifests or versioned wire contracts in erp4us.
---

Required reading: [contracts](../../../docs/engineering/contracts.md), [Open Business Service Protocol](../../../docs/specifications/business-service-protocol.md).

1. Identify the owning version and existing consumers, including UI and integrations. Compare against the historic fixture where applicable.
2. Specify field semantics, optionality, IDs, expected errors and rollout. Keep implementation and positive/negative fixtures aligned.
3. Run `./scripts/quality/check.sh --contracts` and verify implemented owner/consumer behavior separately.

Do not silently repurpose a field, rewrite historical fixtures to make breaking changes pass or edit generated output.

**Done:** compatibility is classified, tests show the actual guarantee and documentation names any migration.
