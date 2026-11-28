---
name: erp4us-engineering
description: Apply the erp4us engineering workflow before any non-trivial code, configuration or architecture change.
---

Required reading: [standards and precedence](../../../docs/engineering/README.md), [principles](../../../docs/engineering/principles.md), [contributor workflow](../../../docs/engineering/agent-workflow.md).

1. Identify the actor, desired outcome, authoritative requirement, and accepted decisions.
2. Check GitHub issues, active PRs and comparable code. Select the skills for every touched boundary.
3. Deliver the smallest complete change, run the applicable [checks](../../../docs/engineering/quality-gates.md), and review the diff.

Never infer acceptance from a draft or bypass a failing gate.

**Done:** the Definition of Done is satisfied; the handoff differentiates passing, blocked and not-run checks.
