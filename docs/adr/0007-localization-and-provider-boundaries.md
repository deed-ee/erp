# ADR 0007: Versioned localization with optional providers

**Status:** Proposed.

**Context:** UI label keys are not a model for translations of business records and documents. Provider access can disclose confidential data.

**Decision:** Use BCP 47 tags and source-revision metadata on translatable resources. Keep source ownership in the owning service. A provider-neutral Translation Service coordinates manual or optional machine candidates and review; approved human translations remain until explicitly superseded. Apply translations under authorization and revision checks. Require explicit provider, purpose and data-class approval before egress; isolate jobs, memory and logs. A manifest can declare resource types but cannot authorize export. Default to source fallback without a provider.

**Trade-off:** Reject frontend-only localization, mandatory AI and unconditional provider submission. Revision and review metadata add overhead, but prevent stale results from silently overwriting approved content. Start with manual localization before adding provider infrastructure.

**Open:** Fallback precedence, retention and residency, ownership of approved document derivatives, and treatment of legacy names whose source locale is unknown.

**Requirements:** [BD-006](../requirements/README.md#bd-006), [BD-014](../requirements/README.md#bd-014).
