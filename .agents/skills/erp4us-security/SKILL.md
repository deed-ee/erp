---
name: erp4us-security
description: Perform defensive review or implement authentication, permissions, tenant isolation and other erp4us security controls.
---

Required reading: [verification](../../../docs/engineering/verification.md), [security policy](../../../SECURITY.md), [threat model](../../../docs/architecture/threat-model.md).

1. Trace trusted identity, grants, inputs and data from ingress to each owning service and store.
2. Review default denial, least privilege and error/log redaction. Add synthetic tests for wrong permission, tenant, organization and applicable stale context.
3. Run the relevant integration, static and advisory checks; request human review for trust changes.

Do not test with production data, publicly disclose findings or weaken security gates.

**Done:** existing trust boundaries are intact, change-specific denials are demonstrated and unresolved risks are reported through the approved channel.
