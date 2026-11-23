# Governance

The repository owner is the bootstrap maintainer. Maintainers review changes, publish decisions, triage issues, and are accountable for security and releases. Over time, nominate additional independent maintainers based on sustained review, contribution and community conduct; announce nominations publicly, allow feedback, and record decisions. Maintainers disclose conflicts and recuse themselves when appropriate. Changes to licensing, branding, governance, production trust boundaries, irreversible architecture or significant costs require explicit maintainer approval.

## RFC process

Open a GitHub Discussion if enabled, otherwise an issue. Summarize the problem, options and tradeoffs, compatibility and migration, security, and measurable acceptance criteria. Discuss publicly and allow reasonable time for feedback. Maintainers record acceptance, rejection or deferral with rationale; accepted architecture decisions become ADRs linked from the proposal. Embargoed security decisions are reviewed privately and documented publicly after remediation where possible.

For new business concepts and industry packages, use the [proposal template](.github/ISSUE_TEMPLATE/proposal.md). Reference affected [requirements](docs/requirements/README.md) and [scenarios](docs/requirements/reference-scenarios.md), or include a new anonymized example with an expected outcome. Describe the owning domain, invariant, alternatives for at least one other industry, and which work belongs in core versus a package. Package proposals also identify the publisher, dependencies and compatibility, permission and data-transfer needs, installation/upgrade/removal plan, and coexistence checks. Industry experts and business users can contribute examples and review outcomes without writing code.

Link the public RFC from the catalog when a proposal is tracked. On disposition, update the catalog and link the decision and follow-up work. When delivering a requirement, link the affected model, contract, implementation and executable evidence before marking it implemented.

Triage with `bug`, `enhancement`, `documentation`, `architecture`, `frontend`, `backend`, `integration`, `security`, `community`, `good first issue` and `help wanted`. Keep the roadmap public. Protect the default branch with PR review and required CI after bootstrap; never auto-merge changes to security controls or architecture. Reviewers verify [DCO sign-off](CONTRIBUTING.md#sign-off-your-contributions) on new contributions before merging. Releases require separate approval.
