---
name: erp4us-refactoring
description: Make focused behavior-preserving changes, reduce duplication or split complex modules in erp4us.
---

Required reading: [principles](../../../docs/engineering/principles.md) and [baseline](../../../docs/engineering/baseline.md).

1. Establish existing behavior with tests; identify active branches that touch the same files.
2. Separate mechanical or formatting changes from functional changes. Simplify responsibilities without inventing generic helpers.
3. Run the same regression checks before and after; apply the normal [check matrix](../../../docs/engineering/quality-gates.md).

Do not change public behavior, storage ownership, wire semantics or security as part of silent cleanup.

**Done:** the diff is narrow, invariants remain tested, and unrelated debt is linked to an existing issue or proposed separately.
