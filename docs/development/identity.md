# Master Data identity configuration

Master Data is an OAuth resource server. It verifies RS256 access tokens against one operator-selected OIDC issuer and resource audience. It does not issue tokens, authenticate passwords or trust scopes from client input. Its local integration fixture uses a throwaway signing key to exercise the same verification path; it is not an IdP.

## Provider and directory choices

The integration boundary is OIDC/OAuth, not Keycloak. The optional loopback preview uses Keycloak as the reference provider so the browser tests exercise real sign-in. An installation can use another compatible provider after verifying its tokens, subject bindings, scopes and key rotation. Entra ID is one candidate, not a verified integration: the issued access-token format and audience must meet the verifier's contract. An LDAP directory can instead sit behind an OIDC provider that supports LDAP federation; erp4us does not bind to LDAP or infer business permissions from directory membership.

The [preview image scan](../../.github/workflows/preview-identity-image.yml) covers the locally built reference image, not the OIDC protocol. It is independent of the backend domain and database proofs. Application CI also scans the image ID used by its real OIDC tests.

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

## Reference image qualification

For [#121](https://github.com/erp4us-org/erp4us/issues/121), an unpublished amd64 candidate was built from the official Keycloak 26.8.2 commit `e514445f9dcdabc5bdcf67cab56c1e49655de067`. The only dependency override was Guava `33.8.0-jre`; the built distribution contained the checksum-verified Maven Central artifact, not a JAR replaced in the official image. The server build used checksum-verified Temurin 21 and Maven 3.9.16; packaging used the upstream UBI layout and digest-pinned UBI bases.

The retested local OCI image ID is `sha256:1a54f8c56f89bc7c41a6e143f34e35be8eadcd1a88e91066f5d0b36f421c75a7`. The unchanged Trivy 0.76.2 gate reported no fixable high or critical finding. The Guava finding was absent; five high findings without a vendor fix remained visible. The CycloneDX SBOM SHA-256 is `678dd07f5207b10828aa6b7e01505d8ea1367321fe4c117f819741237dc9f391`.

The exact scanned image imported a clean realm in a disposable loopback project and passed two real OIDC Playwright cases: authorized NovaMotion planning and rejection of a different tenant's access. This is amd64 evidence for that image, not full receipt-flow or arm64 qualification. The test project was stopped and the original preview containers and volumes were preserved. The local image ID is not a published registry digest.

The maintainer approved using this source and dependency override as a temporary replacement for the synthetic preview and CI. [`compose.preview-identity.yaml`](../../compose.preview-identity.yaml) uses the [small build context](../../infrastructure/preview-identity/) and a local-only image tag. Both CI jobs build from the pinned inputs; the application job scans the exact image ID used in Playwright. Neither job pushes an image. An official fixed digest can replace the temporary recipe only after passing the same image and real OIDC checks. Keep the preview loopback-only; this is not a production artifact or support commitment for other platforms.
