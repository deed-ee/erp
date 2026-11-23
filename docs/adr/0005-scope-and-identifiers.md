# ADR 0005: Explicit scope and stable business identities

**Status:** Proposed.

**Context:** The bootstrap binds Article to one organization and indexes tenant, organization and SKU. This does not model shared records, multiple legal entities or alternative identifiers.

**Decision:** Tenant scope is mandatory. Model legal, operational and reporting scopes independently; verifiable grants and record sharing policies govern access across those dimensions. Use opaque, stable entity IDs. Business identifiers and generated numbers are scheme/scope-specific assignments; normalize and enforce uniqueness under the issuing scheme. Do not silently relax the Article index or coalesce existing records. When introducing a new representation, define a migration or adapter preserving legacy Article IDs and scoped SKU lookups, preflight collisions and verify rollback compatibility.

**Trade-off:** Reject implicit tenant-wide access for records without an organization and reject global business identifiers. Explicit ownership/sharing costs extra policy and index design but avoids collisions and data leakage. Keep one organization-local path for the first order while testing denial across two tenants and organizations.

**Open:** Delegated shared-data stewardship, physical isolation tiers and rules for related legal-party grants and intercompany posting.

**Requirements:** [BD-003](../requirements/README.md#bd-003), [BD-004](../requirements/README.md#bd-004), [BD-010](../requirements/README.md#bd-010).
