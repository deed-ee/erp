---
name: erp4us-persistence
description: Implement or review MongoDB models, scopes, indices, transactions, migrations, retention or restore in erp4us.
---

Required reading: [persistence](../../../docs/engineering/services.md) and [verification](../../../docs/engineering/verification.md).

1. Trace the owner, verified scope, filters, uniqueness, read bounds and concurrency rules at every affected operation.
2. Identify atomic changes and migrations, indexes, compatibility and restore needs before writing an adapter.
3. Run Rust and relevant replica-set checks for rollback, stale edits, replay and cross-scope denial.

Do not use a client-supplied scope, expose storage models as contracts or read another owner's database.

**Done:** live scoped tests and upgrade/rollback evidence match the declared revision and transactional guarantees.
