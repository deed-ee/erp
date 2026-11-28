---
name: erp4us-messaging
description: Implement or review NATS, JetStream, request/reply, commands, queries and durable events in erp4us.
---

Required reading: [contracts and messaging](../../../docs/engineering/contracts.md), [NATS wire specification](../../../docs/specifications/nats-messaging.md), [shared crate](../../../crates/erp4us-messaging/README.md).

1. Draft the language-neutral route, schema, version range, producer, scopes and authorization first.
2. Identify the durable write/outbox/inbox and acknowledgment boundaries; specify retry, duplicate, gap and recovery behavior.
3. Run Rust and contract checks and the relevant real broker integration using disposable credentials.

No second wrapper, alternative subject scheme, unbounded retries or assumption of exactly-once behavior.

**Done:** delivery, denial, replay and outage evidence agree with the declared contract.
