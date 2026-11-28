---
name: erp4us-code-review
description: Perform the final erp4us diff review and record findings and evidence before handoff.
---

Required reading: [review checklist](../../../docs/engineering/review-checklist.md) and [Definition of Done](../../../docs/engineering/agent-workflow.md#definition-of-done).

1. Compare the actual diff to the requested outcome, applicable standards and existing implementations.
2. Look for duplicated logic, broken ownership or trust, complexity, changed contracts and missing failure cases.
3. Check executed evidence and classify actionable findings with path, consequence and a fix or proof.

Do not claim unrun checks, ignore failures, or replace required human approval with agent review.

**Done:** relevant findings are resolved or tracked with a decision, and the handoff lists real results and blockers.
