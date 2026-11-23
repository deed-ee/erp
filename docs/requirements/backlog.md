# Start here for backlog triage

The [capability map](../capabilities/map.yaml) covers the full platform; GitHub tracking is grouped by the business decision or demonstration that can actually be reviewed. Fifteen capability epics supplement existing #16 (Sales, Inventory, Procurement and Workflow), #2 (identity), #4 (messaging) and #18 (operability). This avoids a ticket per noun. The catalog contains the full many-to-many mapping. Do not treat research tickets as ready implementation work.

## Epic issues

- [#39 Capability review: Company scope and delegated access](https://github.com/erp4us-org/erp4us/issues/39)
- [#40 Capability review: Governed master records and business partners](https://github.com/erp4us-org/erp4us/issues/40)
- [#41 Capability review: Product identity and catalog publication](https://github.com/erp4us-org/erp4us/issues/41)
- [#42 Capability review: Quantities and context-specific conversions](https://github.com/erp4us-org/erp4us/issues/42)
- [#43 Capability review: Locale, translation and jurisdiction boundaries](https://github.com/erp4us-org/erp4us/issues/43)
- [#44 Capability review: Approved semantic actions and integrations](https://github.com/erp4us-org/erp4us/issues/44)
- [#45 Capability review: Company documents and lifecycle evidence](https://github.com/erp4us-org/erp4us/issues/45)
- [#46 Capability review: Manufacturing definitions and measured runs](https://github.com/erp4us-org/erp4us/issues/46)
- [#47 Capability review: Quality disposition and material genealogy](https://github.com/erp4us-org/erp4us/issues/47)
- [#48 Capability review: Projects, workforce and service-only delivery](https://github.com/erp4us-org/erp4us/issues/48)
- [#49 Capability review: Legal postings and cost reconciliation](https://github.com/erp4us-org/erp4us/issues/49)
- [#50 Capability review: Asset custody, rental and maintenance](https://github.com/erp4us-org/erp4us/issues/50)
- [#51 Capability review: Shipment and proof of delivery](https://github.com/erp4us-org/erp4us/issues/51)
- [#52 Capability review: Authorized indicators and recovery reporting](https://github.com/erp4us-org/erp4us/issues/52)
- [#53 Capability review: Industry package coexistence](https://github.com/erp4us-org/erp4us/issues/53)

## Task issues

- [#60 Measure the local Article workload and publish a repeatable baseline](https://github.com/erp4us-org/erp4us/issues/60)
- [#61 Add a compatibility check using a previous Article contract fixture](https://github.com/erp4us-org/erp4us/issues/61)
- [#62 Specify and exercise an organization-scoped identifier import](https://github.com/erp4us-org/erp4us/issues/62)
- [#63 Demonstrate stale-revision rejection on one master record](https://github.com/erp4us-org/erp4us/issues/63)
- [#64 Capture local calendar and business-date examples in contracts](https://github.com/erp4us-org/erp4us/issues/64)
- [#65 Define replay semantics for one command key](https://github.com/erp4us-org/erp4us/issues/65)

## Research issues

- [#54 Research: Decide precedence for shared records and local overrides](https://github.com/erp4us-org/erp4us/issues/54)
- [#55 Research: Select a temporal model for retroactive corrections](https://github.com/erp4us-org/erp4us/issues/55)
- [#56 Research: Define conversion and rounding policy at posting time](https://github.com/erp4us-org/erp4us/issues/56)
- [#57 Research: Set translation review and provider boundaries](https://github.com/erp4us-org/erp4us/issues/57)
- [#58 Research: Agree partial-reservation policy for the ten/six/four order](https://github.com/erp4us-org/erp4us/issues/58)
- [#59 Research: Define a scoped number-sequence policy](https://github.com/erp4us-org/erp4us/issues/59)

## Labels and coordination

Use `type:epic/requirement/task/research/rfc` for purpose; existing `bug` and `enhancement` cover bugs and features. `priority:P0` through `priority:P3` express impact, not approval. `status:proposed/needs-analysis/ready/blocked` is the planning state. Use `domain:<ID>` for ownership, existing `backend`, `frontend`, `documentation`, `architecture`, `security`, `integration` and focused `area:api/events/mcp/testing/infrastructure` where useful. `domain expertise needed`, `good first issue` and `help wanted` invite different contributions. Keep independent dependency links even when GitHub allows only one parent.

Existing implementation tickets remain scoped to their original acceptance criteria. Trace links indicate relevant slices, not retroactive approval or scope expansion. In particular, #38 is an Article UI PR, not evidence that registry discovery is implemented. Measurements in #60 establish an observed baseline; the review must choose a supported workload and latency objective before making an availability or performance commitment.
