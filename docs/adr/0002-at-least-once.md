# ADR 0002: Local transactions and at-least-once messages

**Status:** accepted for messaging implementation; not yet implemented.

Use JetStream with a MongoDB transactional outbox and idempotent subscribers. Persist an inbox marker and the resulting state change in the consumer's transaction; do not mark a message consumed on broker receipt. An outbox relay crash after publish but before local completion may republish; stable IDs and consumer-side deduplication are required. Later subscribers define a replay policy and distinguish event time from processing order. Alternatives rejected: direct publish after business writes (loss window) and distributed transactions (coupled failures). No durable event is emitted by the first single-document write; upgrade Compose to replica-set mode before adding the outbox.
