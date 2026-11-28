# Engineering principles

**Purpose and scope.** Make changes understandable and safe across application, website and community repositories, regardless of who or what authors them.

## Mandatory rules

- **Simplicity and reuse.** Before adding a framework, dependency, helper or transport adapter, inspect comparable code. Reuse an existing verified implementation when responsibilities and invariants match. Do not move business concepts into shared technical crates merely to remove repetition.
- **Explicit boundaries.** Make ownership, verified actor/tenant/organization, external inputs, data lifetime and failure paths explicit. Repository and broker policies supplement application authorization; they do not replace it.
- **Correctness and security first.** Preserve domain invariants at the owning service for UI, integrations and agents. Do not trade correct quantities, scope, or durable state for fewer lines or speculative performance.
- **Readability and testability.** Use domain vocabulary consistently (see the [glossary](../glossary/README.md)). Separate business decisions from I/O sufficiently to test rejection and recovery. Keep changes focused and project technical materials in English.

## Recommended and prohibited patterns

Prefer named operations and validated values to positional flags, generic property bags and implicit fallbacks. Use comments to explain constraints or decisions, not restate code. Treat function size, nesting, cyclomatic/cognitive complexity, duplication, cycles and coupling as review signals, not line quotas. Split along responsibilities, not arbitrary lengths. Do not add catch-all helpers, service locators, speculative plugin layers or large frameworks for a local problem.

**Good example:** one owner-side Article registration operation with independent input validation and scoped persistence. **Counterexample:** a separate client implementation that bypasses owner validation or a shared `utils` package containing every service's DTOs.

**Enforcement, exceptions and references.** Native tooling checks mechanical rules; the [review checklist](review-checklist.md) covers semantics. Use the [exception process](README.md#rule-strength-and-exceptions). See the [architecture baseline](../architecture/baseline.md) and [requirements guide](../requirements/guide.md).
