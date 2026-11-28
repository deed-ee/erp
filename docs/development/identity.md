# Master Data identity configuration

Master Data is an OAuth resource server. It verifies RS256 access tokens against one operator-selected OIDC issuer and resource audience. It does not issue tokens, authenticate passwords or trust scopes from client input. Its local integration fixture uses a throwaway signing key to exercise the same verification path; it is not an IdP.

## Provider and directory choices

The integration boundary is OIDC/OAuth, not Keycloak. The optional loopback preview uses Keycloak as the reference provider so the browser tests exercise real sign-in. An installation can use another compatible provider after verifying its tokens, subject bindings, scopes and key rotation. Entra ID is one candidate, not a verified integration: the issued access-token format and audience must meet the verifier's contract. An LDAP directory can instead sit behind an OIDC provider that supports LDAP federation; erp4us does not bind to LDAP or infer business permissions from directory membership.

The [preview image scan](../../.github/workflows/preview-identity-image.yml) covers the pinned reference image, not the OIDC protocol. It is independent of the backend domain and database proofs and must remain in place while that image is shipped.

## Use a self-hosted OIDC provider

1. In a provider such as Keycloak, provision a dedicated realm under HTTPS and set up an OAuth client for the application (Authorization Code with PKCE for browser clients; client credentials for a separate, least-privilege service account). In Keycloak, use an **Audience** protocol mapper on the calling client's scope: set the custom audience to `erp4us-master-data`, enable it for access tokens, and leave it off for ID tokens. Do not reuse the interactive client ID as the API audience.
2. Read the realm's `/.well-known/openid-configuration` over trusted HTTPS. Copy its exact `issuer` into the policy below, and export the public JWKS from its `jwks_uri` to `issuer-jwks.json`. Configure an RSA signing key and RS256 access tokens. The service accepts the standard `at+jwt` header type and Keycloak's `JWT` header plus signed `typ: Bearer` claim. Never supply the issuer's private key to the service.
3. Configure subject bindings using the actual `sub` of each approved principal in that realm. Do not use email addresses or user-editable attributes as identity. Assign a stable internal actor ID and only the tenant/organization permissions needed. Point `ERP4US_IDENTITY_POLICY` and `ERP4US_ISSUER_JWKS` at the two read-only files in the service container.

Example `identity-policy.json`:

```json
{
  "issuer": "https://id.example.invalid/realms/business",
  "audience": "erp4us-master-data",
  "principals": {
    "replace-with-issuer-subject": {
      "actor_id": "242a290f-4bbc-438e-aefe-e3697a8bcd16",
      "grants": [{
        "tenant_id": "ddcae139-c1e8-47d2-8b48-349cb6f30ce5",
        "organization_id": "3e793fb5-75a7-45f6-8f66-45a58fcec674",
        "permissions": ["master-data.article.read", "master-data.article.register"]
      }]
    }
  }
}
```

Send `Authorization: Bearer <access-token>` with the `X-Erp4us-Tenant` and `X-Erp4us-Organization` UUIDs on business requests. Each header selects scope, never grants it. Invalid access tokens return `401`; missing or invalid scope returns `400`; a subject without the exact scope and operation grant receives `403`. For a caller allowed to read the selected scope, IDs outside that scope are indistinguishable from missing IDs (`404`).

Units and partners require their own `master-data.unit.read`, `master-data.unit.register`, `master-data.partner.read`, and `master-data.partner.register` grants. Article grants do not imply access to these resources. The optional Compose Article relay selects only its broker-authorized demo tenant; it leaves other tenants' outbox entries pending. HTTP identity does not grant broker access.

## Rollout and limits

This replaces the demo token before the first release. Existing MongoDB documents need no migration; to retain local demo article access, bind a subject to the same tenant and organization IDs. Stop the previous container before recreating it with the new config. Use short token lifetimes. To rotate keys, make the new public key available alongside the current one and recreate the service before switching signing keys; remove retired keys after outstanding access tokens expire. A membership change takes effect when the service is recreated with the updated policy. There is no automatic JWKS refresh or Organizations service yet. Do not expose the development Compose stack externally; a real deployment needs trusted TLS ingress and operational review.
