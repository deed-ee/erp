---
name: erp4us-angular
description: Implement or review Angular, TypeScript, templates, styling, translations and accessible Solution-First UI in erp4us.
---

Required reading: [frontend](../../../docs/engineering/frontend.md) and [UI patterns](../../../ui/README.md).

1. Start with the decision the user needs to make; locate the owner operation and existing UI primitives and translation keys.
2. Preserve verified scope, handle denied/error/stale results and use stable accessible labels and tokens.
3. Run `./scripts/quality/check.sh --ui`; validate the affected browser workflow, keyboard behavior and axe checks.

Do not add frontend authority, unapproved runtime UI code or alternative business logic for the command bar.

**Done:** equivalent direct paths, context changes and affected locales/layouts are checked and evidence is labeled fixture or live.
