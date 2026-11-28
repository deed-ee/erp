use crate::{
    configured_scope,
    drafts::{Failure, Store},
    material_plan::{STREAM, decode, subject},
    required,
};
use erp4us_messaging::{AckKind, ConnectOptions, Connection, consumer, stream};
use futures_util::StreamExt;
use std::{io, time::Duration};
use tokio::time::timeout;

const DURABLE: &str = "procurement-material-plans-v1";
const ACK_WAIT: Duration = Duration::from_secs(30);
const STORE_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY: [u64; 4] = [1, 2, 4, 8];

async fn connect(user: &str) -> io::Result<Connection> {
    Connection::connect(
        &required("NATS_URL")?,
        ConnectOptions::with_user_and_password(user.into(), required("NATS_PASSWORD")?)
            .custom_inbox_prefix("_INBOX.procurement-inventory")
            .connection_timeout(Duration::from_secs(3)),
    )
    .await
    .map_err(broker_unavailable)
}

fn broker_unavailable<E>(_: E) -> io::Error {
    io::Error::other("Procurement broker operation unavailable")
}

fn durable_config(filter_subject: String) -> consumer::pull::Config {
    consumer::pull::Config {
        durable_name: Some(DURABLE.into()),
        filter_subject,
        deliver_policy: consumer::DeliverPolicy::All,
        ack_policy: consumer::AckPolicy::Explicit,
        ack_wait: ACK_WAIT,
        max_ack_pending: 1,
        // Apply retry limits in the worker: keep redelivering if quarantine is unavailable.
        max_deliver: -1,
        ..Default::default()
    }
}

fn matches_policy(policy: &consumer::Config, filter: &str) -> bool {
    policy.durable_name.as_deref() == Some(DURABLE)
        && policy.deliver_subject.is_none()
        && policy.filter_subject == filter
        && policy.filter_subjects.is_empty()
        && policy.deliver_policy == consumer::DeliverPolicy::All
        && policy.ack_policy == consumer::AckPolicy::Explicit
        && policy.ack_wait == ACK_WAIT
        && policy.max_ack_pending == 1
        && policy.max_deliver == -1
        && policy.backoff.is_empty()
        && !policy.headers_only
}

fn check_policy(info: &consumer::Info, subject: &str) -> io::Result<()> {
    if info.stream_name != STREAM || info.name != DURABLE || !matches_policy(&info.config, subject)
    {
        return Err(io::Error::other(
            "Procurement durable policy does not match configuration",
        ));
    }
    Ok(())
}

pub(super) async fn provision() -> io::Result<()> {
    let scope = configured_scope().map_err(io::Error::other)?;
    let filter = subject(scope).map_err(broker_unavailable)?;
    let broker = connect("provisioner").await?;
    let jetstream = broker.jetstream(Duration::from_secs(5));
    let stream = jetstream
        .operations()
        .get_stream_no_info(STREAM)
        .await
        .map_err(broker_unavailable)?;
    let durable = stream
        .get_or_create_consumer(DURABLE, durable_config(filter.clone()))
        .await
        .map_err(broker_unavailable)?;
    check_policy(durable.cached_info(), &filter)
}

fn retry_delay(delivered: i64) -> Option<Duration> {
    let attempt = usize::try_from(delivered.checked_sub(1)?).ok()?;
    RETRY.get(attempt).copied().map(Duration::from_secs)
}

pub(super) async fn consume() -> io::Result<()> {
    let scope = configured_scope().map_err(io::Error::other)?;
    let filter = subject(scope).map_err(broker_unavailable)?;
    let store = Store::open().await?;
    let broker = connect("procurement_inventory").await?;
    let jetstream = broker.jetstream(Duration::from_secs(5));
    let stream: stream::Stream<()> = jetstream
        .operations()
        .get_stream_no_info(STREAM)
        .await
        .map_err(broker_unavailable)?;
    let durable: consumer::PullConsumer = stream
        .get_consumer(DURABLE)
        .await
        .map_err(broker_unavailable)?;
    check_policy(durable.cached_info(), &filter)?;
    let mut messages = durable
        .stream()
        .max_messages_per_batch(1)
        .messages()
        .await
        .map_err(broker_unavailable)?;
    while let Some(message) = messages.next().await {
        let message = message.map_err(broker_unavailable)?;
        let info = message.info().map_err(broker_unavailable)?;
        if info.stream != STREAM
            || info.consumer != DURABLE
            || info.stream_sequence == 0
            || info.delivered < 1
        {
            return Err(io::Error::other("Unexpected Procurement delivery metadata"));
        }
        let event = decode(
            scope,
            &filter,
            message.subject.as_str(),
            message.headers.as_ref(),
            &message.payload,
        );
        let reason = match event {
            None => "invalid-or-forbidden-envelope",
            // Only quarantine writes are retried beyond the final business attempt.
            Some(_) if info.delivered > RETRY.len() as i64 + 1 => "retry-exhausted",
            Some(event) => match timeout(STORE_TIMEOUT, store.record(&event)).await {
                Ok(Ok(())) => {
                    message.double_ack().await.map_err(broker_unavailable)?;
                    continue;
                }
                Ok(Err(Failure::ConflictingMessage)) => "conflicting-message",
                Ok(Err(Failure::ConflictingPlan)) => "conflicting-plan",
                Ok(Err(Failure::Unavailable)) | Err(_) => {
                    if let Some(delay) = retry_delay(info.delivered) {
                        message
                            .ack_with(AckKind::Nak(Some(delay)))
                            .await
                            .map_err(broker_unavailable)?;
                        continue;
                    }
                    "retry-exhausted"
                }
            },
        };
        match timeout(
            STORE_TIMEOUT,
            store.quarantine(info.stream_sequence, info.delivered, reason),
        )
        .await
        {
            Ok(Ok(())) => {
                tracing::warn!(
                    reason,
                    stream_sequence = info.stream_sequence,
                    "Procurement quarantined an Inventory material plan"
                );
                message
                    .ack_with(AckKind::Term)
                    .await
                    .map_err(broker_unavailable)?;
            }
            Ok(Err(_)) | Err(_) => {
                tracing::warn!("Procurement quarantine unavailable; delivery retained for retry");
                message
                    .ack_with(AckKind::Nak(Some(STORE_TIMEOUT)))
                    .await
                    .map_err(broker_unavailable)?;
            }
        }
    }
    Err(io::Error::other("Procurement consumer stream ended"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use erp4us_messaging::consumer::IntoConsumerConfig;

    #[test]
    fn refuses_durable_with_nonmatching_delivery_policy() {
        let filter = "v1.business.ddcae139-c1e8-47d2-8b48-349cb6f30ce5.inventory.event.material-plan.netted.v1";
        let policy = durable_config(filter.into()).into_consumer_config();
        assert!(matches_policy(&policy, filter));
        for invalid in [
            consumer::Config {
                filter_subject: "v1.business.*.inventory.event.material-plan.netted.v1".into(),
                ..policy.clone()
            },
            consumer::Config {
                filter_subjects: vec![filter.into()],
                ..policy.clone()
            },
            consumer::Config {
                ack_policy: consumer::AckPolicy::None,
                ..policy.clone()
            },
            consumer::Config {
                max_deliver: 3,
                ..policy.clone()
            },
            consumer::Config {
                deliver_subject: Some("_INBOX.other".into()),
                ..policy.clone()
            },
        ] {
            assert!(!matches_policy(&invalid, filter));
        }
    }

    #[test]
    fn business_retry_backoff_is_bounded() {
        assert_eq!(retry_delay(0), None);
        assert_eq!(retry_delay(1), Some(Duration::from_secs(1)));
        assert_eq!(retry_delay(4), Some(Duration::from_secs(8)));
        assert_eq!(retry_delay(5), None);
        assert_eq!(retry_delay(6), None);
    }
}
