# Contributor and agent workflow

The same standards apply to humans and coding agents. Agents MUST load `erp4us-engineering` before a non-trivial change, the applicable skills below before touching a boundary, and `erp4us-code-review` before handoff. Humans follow the same workflow and review checklist without needing an agent.

| Work | Skills to load |
| --- | --- |
| Cargo, Rust code | [Rust](../../.agents/skills/erp4us-rust/SKILL.md) |
| Service bootstrap, boundaries, use cases or diagnostics | [Service](../../.agents/skills/erp4us-service/SKILL.md) |
| Angular, TypeScript UI, templates, CSS | [Angular](../../.agents/skills/erp4us-angular/SKILL.md) |
| NATS, JetStream, commands, queries, events | [Messaging](../../.agents/skills/erp4us-messaging/SKILL.md) |
| HTTP, manifests, descriptors or versioned data | [Contracts](../../.agents/skills/erp4us-contracts/SKILL.md) |
| MongoDB, data models, indexes or migrations | [Persistence](../../.agents/skills/erp4us-persistence/SKILL.md) |
| Identity, authorization, scope, trust or security review | [Security](../../.agents/skills/erp4us-security/SKILL.md) |
| Tests, fixtures, replay or coverage | [Testing](../../.agents/skills/erp4us-testing/SKILL.md) |
| Docker, Compose, CI or dependencies | [Infrastructure](../../.agents/skills/erp4us-infrastructure/SKILL.md) |
| Behavior-preserving cleanup | [Refactoring](../../.agents/skills/erp4us-refactoring/SKILL.md) |
| Non-trivial changes / handoff | [Engineering](../../.agents/skills/erp4us-engineering/SKILL.md) / [Code review](../../.agents/skills/erp4us-code-review/SKILL.md) |

## Discovery and trust

`.agents/skills/` is the only source. [OpenCode V2](https://opencode.ai/docs/skills) and [Codex](https://developers.openai.com/codex/build-skills) document project discovery there. [GitHub Copilot](https://docs.github.com/en/copilot/concepts/agents/about-agent-skills) documents it for supported surfaces; support varies by client. [Claude Code](https://code.claude.com/docs/en/skills) documents `.claude/skills/`, not `.agents/skills/`; `CLAUDE.md` points to the same instructions without copying skills. If discovery is absent or blocked, read the linked `SKILL.md` directly. Start at the repository root to pick up project instructions. Do not assume auto-discovery proves adherence.

Use the [Agent Skills specification](https://agentskills.io/specification). Skills MUST stay concise, link canonical standards via relative Markdown links and avoid installing tools or executing remote scripts. External skills require source, license, content, trust and version review before any proposed import. No external skills are needed here.

## Definition of Done

- Understand the actor, outcome, requirement, ADRs and trust boundaries. Inspect comparable code, linked issues and active PRs; resolve overlaps before editing. Read the applicable standards and skills.
- Deliver a focused, working change without duplicating infrastructure or logic. Preserve existing behavior where promised; document contract, permission, migration, rollback, dependency and operational implications.
- Add meaningful positive and negative evidence. Run applicable [quality checks](quality-gates.md), including formatting, lint, build and contract checks. Do not bypass a gate. Explicitly record not-run or blocked checks and why.
- Review the final diff against the [checklist](review-checklist.md). Use an independent review context for significant or security-sensitive changes where available; it cannot replace human or maintainer approval.
- Update relevant documentation and the PR (or existing issue when no PR exists). Link checks, the diff and outstanding decisions. Sign off your own commits according to the [DCO policy](../../CONTRIBUTING.md#sign-off-your-contributions).
