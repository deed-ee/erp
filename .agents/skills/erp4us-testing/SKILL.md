---
name: erp4us-testing
description: Add or evaluate erp4us unit, integration, contract, coverage, browser and recovery tests.
---

Required reading: [verification and testing](../../../docs/engineering/verification.md), [check matrix](../../../docs/engineering/quality-gates.md).

1. Name what each assertion proves. Choose the lowest level that can establish it.
2. Cover success and applicable denial, stale input, duplicate, outage and recovery. Use bounded assertions and isolated real dependencies at boundaries.
3. Run changed suites and compare relevant exercised coverage, not just a global percentage.

Do not accept a fully mocked business E2E as proof of live owner behavior or claim unrun results.

**Done:** evidence states its scope, fixtures, expected failures and remaining limitations.
