# Continuous deployment and public demonstration

The public demo is a maintained ERP4us delivery target, starting with the first functional slice that passes the public exposure gate. Today, Compose and GitHub Actions validation exist; there is no hosted demo, Helm chart, GHCR publishing pipeline or Argo CD deployment. The local demo bearer credential grants writes and must never be exposed publicly. This design does not authorize provisioning infrastructure, activating a public endpoint, or purchasing services.

## Environments

| Environment | Deployment and data | Access |
|---|---|---|
| Local development | Docker Compose; locally built images and disposable data | Loopback by default; local credentials; synthetic data only with demo auth |
| Continuous development | Kubernetes/k3s, Helm and Argo CD; deploy merged `main` builds by digest; disposable scenario fixtures | Contributor/operator access only; no public ingress |
| Public demo | Kubernetes/k3s, Helm and Argo CD; promote the tested build and versioned dataset | TLS public ingress to guided reads only at launch; separate staff access for operations |

Development and demo use separate namespaces, service accounts, credentials and secret references, MongoDB databases/users, S3 buckets/policies, and NATS accounts or equivalently isolated JetStream streams and subject permissions. Separate PVCs and backups where applicable. Neither environment's principal can read or mutate the other's data; never import production snapshots. A shared cluster is acceptable only if the CNI enforces NetworkPolicy and isolation tests pass. Use default-deny ingress/egress with explicit paths, least privilege and restricted pod security (non-root, no privilege escalation, drop capabilities, read-only root filesystem where supported). Set pod requests/limits, namespace quotas and readiness, liveness and startup probes as needed. Do not expose datastores, management ports or Argo CD publicly. Install and configure only the dependencies used by the current slice; no JetStream, S3 or extra controllers merely to fill the chart.

## Build once, promote by digest

| Trigger | GitHub Actions and GitOps outcome |
|---|---|
| Pull request | Run language, contract, integration, and dependency checks; when charts land, lint/render both environments and validate policy, migration and seed changes. Build and scan test images without publishing, deployment credentials or environment mutation. |
| Merge to `main` | Build and scan the final images, publish to GitHub Container Registry (GHCR), and record digests, commit, SBOM/provenance, chart, migration and fixture versions. Update checked-in development desired state to those digests. Argo CD reconciles and verifies readiness; Actions does not need cluster admin access. |
| Validated development build | Run post-sync smoke and applicable scenario/E2E checks against the exact release tuple. Promote that tuple to the public-demo desired state without rebuilding or resolving tags again. Protect promotion with checks and authorized review; serialize updates so older jobs cannot supersede newer deployments. Confirm public smoke checks after sync. |
| Approved release | Publish versioned, deployable charts and signed/pinned image artifacts, with configuration, migration, backup/restore and upgrade notes. No automatic production installs or release approvals inherited from demo promotion. |

Pin container image references, including dependency images, to OCI digests in Kubernetes. Keep Helm templates in version control with distinct values and Argo CD applications/projects scoped to each destination. Keep the last known-good deployment tuple. A failed gate leaves the demo on its previous build; for a bad rollout revert desired-state Git, verify recovery, and never assume a chart rollback reverses a database migration. Use forward-compatible expand/contract migrations or an explicitly tested restore or roll-forward plan. Keep automated pruning away from persistent data unless an operator explicitly authorizes it.

## Public modes and launch gate

1. **Guided read-only first.** Offer predefined synthetic companies, a guided browse path and conspicuous feature availability labels. The earliest milestone can show seeded Article details, without waiting for Angular, order-to-procurement or all six industry workflows. Use a minimal read-only surface (for example a guided UI backed by narrowly scoped reads). Deny writes, tenant selection outside the public catalog, admin routes and agent commands at the application and ingress; grant the serving identity read-only storage access. Seed/migration identities run privately and separately. Verify anonymous reads work, all write attempts fail and there is no path to development credentials or data before opening ingress.
2. **Temporary writable sandbox later.** Use verified session-to-tenant binding and the existing service authorization model; never share the guided demo tenant or infer tenant from input. Allocate a separate seeded tenant with an expiry, limit tenant creation and write rate, and cap record count, storage, request size, concurrency and workload. Enforce expiry on access even if cleanup has not run. Revoke sessions and asynchronously remove scoped DB state, objects and applicable messages or derived state with retry, audit and failure alerts. Test two-sandbox isolation, expiry during in-flight work, interrupted cleanup and quota enforcement before enablement. Do not accept real business data.

Both modes disable real external integrations, payments, outbound mail and uncontrolled AI provider use. Allow only simulated adapters or explicitly approved synthetic-only capabilities, enforce the restriction in service policy and on the network, and do not forward user content. Limit public request rates. Keep a documented emergency disable path for ingress and promotion.

## Reproducible data and operations

The [reference datasets](../requirements/reference-scenarios.md#demo-fixtures) are repository inputs for public walkthroughs, integration fixtures, E2E tests, documentation and architecture reviews. Scenario IDs remain stable. Version fixtures and use stable logical IDs, dates and commands; any server-generated IDs/timestamps must be mapped back to stable fixture keys rather than asserted verbatim. Extend each scenario as the corresponding business capability ships; do not imply that planned fields or workflows are implemented.

Version migrations separately from fixtures. A private deployment job verifies the environment and target tenant, applies migrations in order, and loads the requested fixture version idempotently. Reset is explicit and scoped: rebuild only the designated synthetic target; verify fixture keys/counts and record the applied version. A failed seed must not mark rollout ready. Test replay and reset on disposable data; never expose reset publicly or target a production tenant/database. Service-owned schemas and invariants still apply.

Public launch requires a runbook for local and approved cluster setup, GitOps promotion and rollback, migration/seed/reset, troubleshooting, and incident ownership. Start with structured redacted logs, liveness/readiness, deployment status, synthetic availability checks, and basic request/error/latency and resource metrics. Alert maintainers on repeated sync or seed failures, unavailable demo, or cleanup failures. Each change affecting a demoed workflow updates the fixture, walkthrough and verified checks together. Promote from the normal pipeline rather than editing the running site manually.
