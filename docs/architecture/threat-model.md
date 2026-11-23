# Initial threat model

Protect business documents, personal data, tenant boundaries, inventory and financial integrity, credentials, and audit history. Assume hostile inputs from browsers, API clients, installed integrations, documents, and agent tool results. The local Docker host and operators with control of it are trusted.

| Boundary | Main risks | Required controls |
|---|---|---|
| Internet to gateway and services | Credential theft, confused deputy, tenant/object access, resource abuse | TLS, verify issuer/audience/expiry against trusted keys; validate membership and permission for each operation in the owning service, including lookups and exports. Scoped rate limiting, deny by default, negative tests across two tenants and organizations. |
| Services to MongoDB, JetStream and S3 | Cross-service writes, spoofed actor/event, poison messages, broad object access | Dedicated credentials and subject/bucket policies, validate envelope and producer permissions, tenant-scoped filters, controlled replay, private objects with short-lived grants, no browser NATS credentials. |
| Operator to registry/runtime | Malicious manifests, arbitrary network fetches, dependency substitution and executable UI | Reviewed identities and digests, verified registration, offline resolution within vetted bundles, declarative UI only, least-privilege deployment separate from web app; no Docker socket or cluster admin. |
| Agent and documents to business actions | Prompt injection, unsafe writes, forged approvals or data exfiltration | Treat all retrieved content as data, tools allowlisted and scoped per actor, separate queries from commands, preview and explicit approval bound to command arguments and revision, bounded execution, auditable decisions. |
| CI to release artifacts | Secrets in builds or logs, compromised dependencies, mutable artifacts | Read-only PR tokens, pin dependencies/actions/images, vulnerability checks, restricted release privileges, SBOM and provenance before release. |

## Current exposure and limits

Compose publishes only the API on loopback; MongoDB has no host port. Use synthetic data on a trusted local machine only. The API validates signed access tokens and checks both scope selectors and an operation-specific permission against an operator-managed subject binding. Lookup queries use both scope fields. Trusted configuration and the issuer signing keys are part of the authorization boundary; never use fixture keys outside local tests. `.env` is visible to trusted host/Docker operators and must not be shared. The service does not fetch keys from token-supplied URLs. Key and grant changes require replacing the files and recreating the service; there is no real-time token revocation, online key rotation, TLS termination, audit trail, rate limiting, or disaster recovery. Before non-local deployment, review these operational risks and validate migration, backup and recovery.
