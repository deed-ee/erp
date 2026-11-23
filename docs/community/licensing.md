# Licensing decision

On 2026-11-23 the maintainer approved [Apache-2.0](../../LICENSE) for first-party code, documentation, specifications and examples, and [DCO 1.1 sign-off](../../CONTRIBUTING.md#sign-off-your-contributions) for contributions. See [decision #1](https://github.com/deed-ee/erp4us/issues/1). A single permissive license with an explicit patent grant simplifies adoption and independent integrations.

## Infrastructure and toolchain

- MongoDB Community Server uses the [SSPL](https://www.mongodb.com/legal/licensing/server-side-public-license), which is **not OSI-approved**. The official Rust driver is Apache-2.0. Self-hostable does not mean every infrastructure component is OSI-licensed; distributing the server or offering it as a service requires its own license review.
- NATS Server and common self-hosted OIDC candidates such as Keycloak use Apache-2.0; Helm/Kubernetes are Apache-2.0. S3 is an interface, not a license: evaluate the exact storage distribution (e.g. Apache-2.0 SeaweedFS vs. AGPL-licensed distributions), features and packaged dependencies before choosing.
- Rust is generally dual MIT/Apache-2.0; Tokio, Axum and Angular use permissive terms. OCI base images, Debian packages, binaries in vendor images, Docker distributions and third-party frontend assets each require a component inventory and their own notices. GitHub Actions/GHCR are optional for development automation and image hosting, not runtime requirements.
