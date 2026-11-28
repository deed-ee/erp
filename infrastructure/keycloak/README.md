# erp4us Keycloak login theme

`themes/erp4us-v1` is a presentation-only theme for the login templates in Keycloak `26.8.2`, the version used by the Manufacturing Preview draft. The underlying provider retains its templates, form actions, credential entry, redirects, session handling, localization and configured authentication flows. The theme changes no realm defaults, clients, privileges, cookies or authentication code. It does not run in the public Astro site or collect credentials in Angular.

The desktop layout pairs the existing erp4us symbol with a branched-process illustration in muted slate and teal. The illustration is decorative, has no controls or information, and disappears on narrow screens. The native provider's locale selector, sign-in and conditional recovery controls retain their position in the document and their native labels and focus order. Fonts and decorative assets load locally; the theme introduces no external requests. Keep `erp4us-symbol.svg` identical to the approved `ui/public/brand/symbol.svg`.

## Test without touching the Preview

Use Node 24 with the **existing** UI dependencies and Chromium installed (`npm ci && npx playwright install chromium` in `ui/`). With the exact pinned image already available locally, run from the repository root:

```sh
sh tests/integration/keycloak-theme.sh
```

The test starts a separate disposable provider with random synthetic credentials, a temporary local TLS certificate and a `theme-check` realm on `127.0.0.1:19443`, with a test callback on `127.0.0.1:19444`. `ERP4US_THEME_PORT` and `ERP4US_THEME_CALLBACK_PORT` can select different free ports. It does not reseed the Preview realm or reuse its volume, credentials or routes. The test browser ignores the generated certificate only in this dedicated suite. The harness checks the approved mark, provider-native login, rejected credentials, an authorization-code redirect with matching state using a PKCE client, mobile language and recovery navigation, required-password-update rendering, and automated accessibility. It does not exchange the code, send password reset email, cover the Angular session or certify WCAG conformance. Browser captures are written under `ui/test-results/keycloak-theme/`.

The harness does not enable, exercise or claim external identity providers, configured MFA, passkeys, or expiry handling. They remain controlled by the provider and must be verified against the approved deployment configuration before activation.

Review actual browser captures from the disposable provider: [English desktop](screenshots/sign-in-desktop.png) and [German mobile](screenshots/sign-in-mobile.png).

## Integration and rollback boundary

No Preview or production compose file activates this theme. When the integrated identity path is ready for review, bind-mount the versioned directory read-only at `/opt/keycloak/themes/erp4us-v1` and select `erp4us-v1` for the intended realm's **Login theme**, without changing its authentication configuration. Return to the previous theme to roll back; do not recreate or delete an existing realm to change presentation. Recheck inherited CSS, redirects, re-authentication, configured second factors, localization and accessibility on each supported upstream version.

[#84](https://github.com/erp4us-org/erp4us/issues/84) tracks the separate Angular sign-in entry and route return. [#121](https://github.com/erp4us-org/erp4us/issues/121) still governs image qualification. A passed browser smoke test using this loopback-only, synthetic fixture is **not** a security qualification, release, or change to the pinned artifact.
