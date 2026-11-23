# ADR 0002: Local transactions and at-least-once messages

**Status:** accepted; local transactional outbox implemented for Article registration.

Use JetStream with a MongoDB transactional outbox and idempotent subscribers. Persist an inbox marker and the resulting state change in the consumer's transaction; do not mark a message consumed on broker receipt. An outbox relay crash after publish but before local completion may republish; stable IDs and consumer-side deduplication are required. Later subscribers define a replay policy and distinguish event time from processing order. Alternatives rejected: direct publish after business writes (loss window) and distributed transactions (coupled failures).

Master Data commits the article and the [`ArticleRegistered` envelope](../../contracts/schemas/article-registered-v1.json) in a single transaction on the Compose replica set. The stored UUID is the stable message ID and also starts the correlation chain. The payload identifies the article and revision without including the SKU or name. Existing articles are not backfilled. A broker, relay, and consumer are not present yet: the outbox is written but not drained or published.
