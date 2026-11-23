# ADR 0002: Local transactions and at-least-once messages

**Status:** accepted; Article registration is exercised across two local services.

Use JetStream with a MongoDB transactional outbox and idempotent subscribers. Persist an inbox marker and the resulting state change in the consumer's transaction; do not mark a message consumed on broker receipt. An outbox relay crash after publish but before local completion may republish; stable IDs and consumer-side deduplication are required. Later subscribers define a replay policy and distinguish event time from processing order. Alternatives rejected: direct publish after business writes (loss window) and distributed transactions (coupled failures).

Master Data commits the article and the [`ArticleRegistered` envelope](../../contracts/schemas/article-registered-v1.json) in a single transaction on the Compose replica set. The stored UUID is the stable message ID and also starts the correlation chain. The payload identifies the article and revision without including the SKU or name. Existing articles are not backfilled.

The optional local delivery stack uses one Master Data relay and a separate Article Receipts consumer with its own database and credentials. The relay records `publishedAt` only after a JetStream acknowledgement. The consumer atomically upserts a single document keyed by message ID: the document is both the receipt and inbox marker, so a replay cannot create a second receipt. Invalid events are recorded by stream sequence without retaining their payload in the database, then terminated. The original message stays in JetStream for operator inspection.

This slice handles initial registration only (revision 1). The relay scans by creation timestamp and MongoDB key, but that is not a business ordering guarantee; the receipt does not depend on other events. The stream is size-limited and rejects new writes when full instead of evicting old messages. The relay leaves those writes pending and retries; there is no automated stream retention, pressure alert, operator repair/replay interface or coordinated broker/database recovery. This is not a production deployment.
