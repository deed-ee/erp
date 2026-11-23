# ADR 0003: Layer semantics over existing standards

**Status:** accepted for drafting, no stability promise yet.

Use OpenAPI 3.1 for HTTP and JSON Schema 2020-12 for payload shapes. Add a language-neutral service manifest and semantic descriptor for intent, permission and localization references that those standards do not canonically model. Metadata is never trusted to grant a permission, execute code, or implement an invariant. Avoid reproducing schema fields per UI component and inventing a workflow language. The first descriptor is validated against the API, but registry activation, dynamic rendering and MCP generation are separate deliverables.
