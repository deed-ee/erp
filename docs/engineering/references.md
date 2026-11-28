# Reference implementations

These are slices of tested code, not templates. Read the linked test and the standard before adapting a slice. Do not treat known limitations or local fixture assumptions as approved defaults.

## Rust

| Pattern | Source | Evidence and limits |
| --- | --- | --- |
| HTTP bootstrap and signal drain | [Master Data `main.rs`](../../services/master-data/src/main.rs) | [Smoke and restart](../../tests/integration/smoke.py); use the binding and shutdown shape, not its dev deployment settings. |
| Domain validation and error mapping | [Article domain](../../services/master-data/src/domain.rs), [HTTP adapter](../../services/master-data/src/http.rs) | [Live create, duplicate and reject cases](../../tests/integration/smoke.py); keep wire errors in the adapter. |
| Verified scope and MongoDB repository | [Authentication](../../services/master-data/src/auth.rs), [store](../../services/master-data/src/store.rs) | [Partner scope assertions](../../tests/integration/partners.py), [owner restart and outbox](../../tests/integration/smoke.py). Local issuer/key rotation remains limited. |
| Small worker and scoped deduplication | [Procurement](../../services/procurement/src/main.rs), [store](../../services/procurement/src/store.rs) | [Replica-set draft and replay](../../tests/integration/procurement.py). This is the main-branch worker, not the Preview purchase API. |
| Durable publication, consumption and bounded quarantine | [Article Delivery](../../services/article-delivery/src/main.rs), [transport library](../../crates/erp4us-messaging/src/client.rs) | [Delivery, outage and recovery tests](../../tests/integration/delivery.py). Do not copy the entire multi-role binary as boilerplate. |

## Contracts and reliability

- [Article OpenAPI](../../contracts/openapi/master-data-v1.yaml), [descriptor](../../contracts/entities/article-v1.json), [manifest](../../contracts/manifests/master-data-v1.json) and [old reader](../../tests/contracts/fixtures/article-reader-v1.json): [contract validator](../../tests/contracts/check.py) checks this declared subset of compatibility.
- [Article wire example](../../contracts/examples/nats/article-registered-v1.json), [Rust wire fixtures](../../crates/erp4us-messaging/tests/wire.rs), [real broker test](../../crates/erp4us-messaging/tests/broker.rs) and [independent Python configuration client](../../tests/integration/config.py): test syntax, cross-field behavior and actual broker restrictions separately.

## Web

- [Products](../../ui/src/products.ts) and [managed controls](../../ui/src/ui.ts), with [Article workflows](../../ui/tests/article.spec.ts) and [shell visuals and axe](../../ui/tests/shell.spec.ts): use the UI composition and translation keys. The [Article adapter](../../ui/src/article-api.ts) is a wiring reference, not evidence that all response bodies get runtime decoding.
- No stable main-branch structured-result renderer or Keycloak theme is indexed yet. Review candidate patterns in [#131](https://github.com/erp4us-org/erp4us/pull/131) only when adopting that feature; the theme is not activated in the Preview.

## Tooling

- [Existing CI](../../.github/workflows/ci.yml), [CI runbook](../development/ci.md), [Master Data runtime stages](../../services/master-data/Dockerfile) and [Compose overlay](../../compose.delivery.yaml): preserve pinned build dependencies, narrow permissions and fixture isolation.
- A scaffold would need an approved second consumer and a tested owner contract; use the small/large profiles in [services](services.md) instead of adding a generator at this stage.
