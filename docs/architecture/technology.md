# Technology compatibility

| Area | Bootstrap choice | Next evaluation |
|---|---|---|
| Service | Rust 1.100.0, Tokio, Axum 0.8, Serde, tracing; workspace lockfile and direct dependency pins | Extract minimal SDK only after reuse is proven; add OpenTelemetry and contract-linked traces. |
| Persistence | MongoDB 8.0 and official Rust driver 3.10; authenticated local standalone for a single-document insert | Verify replica-set transactions, bootstrap/migration privileges, and backup/restore. Compare compatible alternatives only through an ADR and acceptance suite. |
| Messaging | JetStream and `async-nats` selected, not installed | Pin versions after verifying broker/client compatibility, auth, reconnect, durable pull consumers and failure tests. |
| Browser | Angular baseline as defined in the architecture, no dependencies installed | Pin an Angular/CDK-aligned release, supported Node/TypeScript, Tailwind, Lucide, and Playwright in a verified build matrix; pick ECharts only once required. |
| Identity and files | OIDC/OAuth 2.0 and S3 API selected, no distribution installed | Verify a self-hostable IdP and an S3 implementation with signed URLs and checksums; review image licenses and hardening. |
| Deployment | Compose with pinned image digests, GitHub Actions validation | [Helm and Argo CD](deployment.md) for Kubernetes/k3s, scanning and GHCR digest promotion. Verify Kubernetes versions and architecture support before advertising them. |

Record each candidate's upstream support window, license, vulnerability policy, and compatibility checks before pinning. Keep deployment independent of commercial services; GitHub automates project development but is not a runtime dependency.
