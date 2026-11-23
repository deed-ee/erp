# UI foundation requirements

Stable UI IDs supplement the business catalog proposed in [#31](https://github.com/erp4us-org/erp4us/pull/31); its BD-003, BD-004, BD-006, BD-017, BD-018 and BD-021 remain authoritative for business scope, translation, traceability and demo policy once accepted.

| ID | Requirement and acceptance | First slice |
|---|---|---|
| <a id="ui-001"></a>UI-001 Identity | Use lowercase `erp4us`, the geometric four and subdued teal/slate. Version light/dark wordmarks, symbol, favicon, app icon, avatar and social card. Apply the same approved assets in the site and app. | Reviewable assets; live profile publication pending. Coordinate with [#35](https://github.com/erp4us-org/erp4us/issues/35). |
| <a id="ui-002"></a>UI-002 Design system | Reuse semantic tokens and documented native controls in every module. Add components when a tested workflow needs them. | Tokens and initial recipes. |
| <a id="ui-003"></a>UI-003 Shell and scope | Keep landmarks, navigation, context and status consistent; switch only to verified scope and preserve edits on cancellation or rejection. Report only known environment/health data. | Fixed demo context and focus management; verified scope depends on [#2](https://github.com/erp4us-org/erp4us/issues/2). |
| <a id="ui-004"></a>UI-004 Module navigation | Render enabled, installed, permitted entries using stable, localized and versioned metadata; unimplemented routes cannot be selected. | Build-time module registration. Coordinate runtime discovery with [#3](https://github.com/erp4us-org/erp4us/issues/3). |
| <a id="ui-005"></a>UI-005 Workspace | Open a personal landing page with real quick actions; show unavailable widgets without inventing task, approval or KPI data. | Product entry point and explicit unavailable states. |
| <a id="ui-006"></a>UI-006 Business lists | Share page controls and typed bounded server-side queries. Verify permission-aware filtering/sorting, cursor behavior and clear empty/error states. | Product route with ID lookup and explicit absence of listing. |
| <a id="ui-007"></a>UI-007 Entity views | Share title, identifier, revision, forms and sections. Use service validation and preserve unsaved input on conflicts/cancelled navigation. | Register/get with a deep link and duplicate handling. Coordinate [#3](https://github.com/erp4us-org/erp4us/issues/3) and [#34](https://github.com/erp4us-org/erp4us/issues/34). |
| <a id="ui-008"></a>UI-008 Contextual panels | Register panels by trusted IDs and permissions; support keyboard opening/closing and avoid obscuring main content. | Product help panel. |
| <a id="ui-009"></a>UI-009 Accessibility | Meet WCAG 2.2 AA with automated and manual evidence; verify keyboard, focus, reflow, labels, status announcements and contrast without relying on color alone. | Automated first-slice checks; manual conformance review pending. |
| <a id="ui-010"></a>UI-010 Locale and preferences | Persist only display settings. Localize with stable keys and fallback; use locale-aware number, currency and date formatting with explicit time zones and allow future RTL. | English/German, system/light/dark, density, panel and secondary nav preferences. |

## Issue drafts for publication

- **Epic: Mature the erp4us UI/UX foundation** (UI-001–UI-010). Review the first shell and website together. Confirm the branded asset rollout, signed-off UX decisions and compatibility with #2, #3, #15, #31, #34 and #35. Track the next items below without reopening covered work.
- **Add bounded Product browsing to Master Data** (UI-006). Publish a scoped list API with a stable tie-breaking sort, documented filter allowlist, opaque cursor, explicit limits and tenant-isolation tests. Wire a shared grid only after the contract exists; verify next page, empty results and errors.
- **Define revision-checked Product editing and retirement** (UI-007). Agree on update and lifecycle/retirement use cases before promising physical deletion. Publish permission, revision and conflict semantics and test stale-write rejection. UI keeps edits and allows safe reconciliation without optimistic overwrite.
- **Complete the first UI accessibility review** (UI-009). Verify light/dark/compact, tab order, focus return, screen-reader validation announcements, zoom/reflow and target sizes on app and website. Keep regression baselines pinned to the browser and OS in CI.
