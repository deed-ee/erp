---
name: erp4us-infrastructure
description: Change or review Docker, Compose, CI workflows, shell tooling and dependency or deployment configuration in erp4us.
---

Required reading: [infrastructure](../../../docs/engineering/infrastructure.md) and [CI runbook](../../../docs/development/ci.md).

1. Identify the trust context, pinned toolchain, data/secrets boundary and existing jobs before adding anything.
2. Preserve required and Preview-only checks, validate workflow inputs and cache provenance, and plan rollback.
3. Run `./scripts/quality/check.sh --infra`; for behavior changes run the relevant disposable runtime job.

Do not execute untrusted PR code in privileged contexts or add a parallel scanner/workflow for an existing check.

**Done:** native validation and appropriate hosted evidence are recorded; no deployment or repository policy changed implicitly.
