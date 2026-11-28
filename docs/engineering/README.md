# Engineering standards

One project, one engineering standard, many contributors and agents. This directory in `erp4us-org/erp4us` is the canonical home for engineering standards. Repository instructions and skills explain how to apply them; they do not create additional rules.

## Precedence

1. Accepted architecture decisions and security policies: [ADRs](../adr/), the [threat model](../architecture/threat-model.md), [security policy](../../SECURITY.md), and applicable organization policies.
2. [Engineering principles](principles.md).
3. Language and technology standards: [Rust](rust.md), [TypeScript and Angular](frontend.md), [infrastructure and tooling](infrastructure.md).
4. Domain and service standards: [services, persistence and operations](services.md), [contracts and messaging](contracts.md), [security and testing](verification.md).
5. [Agent and contributor workflow](agent-workflow.md).
6. [Reference implementations](references.md).
7. [Automated quality gates](quality-gates.md).
8. Individual implementation.

Accepted decisions govern. A proposed ADR, RFC, draft contract or feature branch does not grant authority or prove a capability. Existing published contracts remain authoritative for their interfaces. If documentation, a reference or a gate disagrees with a higher-level rule, stop and resolve the conflict rather than copying the example or bypassing the gate.

## Rule strength and exceptions

- **MUST / MUST NOT:** mandatory / prohibited.
- **SHOULD / SHOULD NOT:** recommended / discouraged; explain a material deviation in the PR.
- **MAY:** permitted; not a requirement.

For an exception to a mandatory engineering rule, record the rule and scope, rationale, alternatives, risk, compensating controls, owner and expiry or removal criterion in the PR and linked GitHub issue. Obtain maintainer approval before merge; use a private security record if needed. A lint allowance belongs at the narrowest scope with a reason and a link when the risk or remediation extends beyond the diff. Exceptions cannot waive critical security controls or override accepted architecture/security decisions. Consequential changes still require the [RFC and approval process](../../GOVERNANCE.md#rfc-process).

## Adoption and maintenance

Start with the [contributor workflow and Definition of Done](agent-workflow.md), [review checklist](review-checklist.md), [quality commands](quality-gates.md), and [initial baseline](baseline.md). Use existing GitHub issues for debt and review related PRs before opening new ones. Record enforcement gaps explicitly; a review-only rule is still a rule.

The website and community repositories reference a reviewed application commit rather than copying these documents. Local runbooks and technology configuration remain with their owning repository. Update a pin in a reviewed PR, verify relevant local checks and links, and retain the previous pin on failure. Do not fetch or execute policy tooling at build time from a mutable external branch.

When adding a language, framework or major pattern, explain its concrete use case, ownership, maintenance and security risk. Bring the applicable standard, skill guidance, real example and executable checks with it, or record a time-bounded adoption plan. Keep standards, tools and the [reference index](references.md) together when they change.
