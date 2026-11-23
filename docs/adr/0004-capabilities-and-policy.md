# ADR 0004: Typed capabilities and versioned extension policy

**Status:** Proposed.

**Context:** Partners and products participate in multiple workflows. Industries need extensions that can coexist without forking the platform.

**Decision:** Use one stable identity per entity with composable, versioned capability and role contracts. Assign schema, validator, permission and behavior ownership explicitly. Use approved, namespaced attribute definitions and deterministic, typed extension points. Industry Packages declare versions, dependency/compatibility ranges, migrations, scope and permissions. Validate a package and its conflicts before activation; operator approval cannot give it authority beyond service policies. Treat process configuration as versioned inputs to business commands, never as executable logic or permission grants.

**Trade-off:** Reject exclusive subtype hierarchies, per-industry forks and arbitrary JSON rules. Typed extension points cost more to introduce but make validation, upgrades and historical behavior reviewable. Preserve resolved schema and policy versions across deactivation.

**Open:** Package signing/distribution, conflict rules and migration rollback for actively referenced definitions. Prove a declarative package pilot before choosing plugin execution or workflow runtime.

**Requirements:** [BD-002](../requirements/README.md#bd-002), [BD-005](../requirements/README.md#bd-005), [BD-014](../requirements/README.md#bd-014), [BD-015](../requirements/README.md#bd-015).
