use async_nats::jetstream::{self, AckKind, Context, consumer, stream};
use futures_util::StreamExt;
use mongodb::{
    Client, Collection,
    bson::{Document, doc},
};
use serde::Deserialize;
use std::{env, io, time::Duration};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

const SUBJECT: &str = "business.master-data.article.registered.v1";
const STREAM: &str = "ARTICLES";
const CONSUMER: &str = "article-receipts";
const PAUSE: Duration = Duration::from_secs(1);
const RECEIPT_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];
const MAX_RECEIPT_DELIVERIES: i64 = 4;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    message_id: Uuid,
    message_type: String,
    schema_version: u32,
    timestamp: String,
    tenant_id: Uuid,
    organization_id: Uuid,
    source_service: String,
    actor: Actor,
    correlation_id: Uuid,
    traceparent: Option<String>,
    payload: Payload,
}

#[derive(Deserialize)]
struct Actor {
    id: String,
    kind: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    article_id: Uuid,
    revision: u32,
}

impl Event {
    fn valid(&self) -> bool {
        self.message_type == SUBJECT
            && self.source_service == "erp4us.master-data"
            && self.schema_version == 1
            && self.payload.revision == 1
            && OffsetDateTime::parse(&self.timestamp, &Rfc3339).is_ok()
            && !self.actor.id.is_empty()
            && matches!(self.actor.kind.as_str(), "user" | "service" | "agent")
            && self.traceparent.as_deref().is_none_or(valid_traceparent)
    }

    fn receipt(&self) -> Document {
        let mut receipt = doc! {
            "articleId": self.payload.article_id.to_string(),
            "tenantId": self.tenant_id.to_string(),
            "organizationId": self.organization_id.to_string(),
            "revision": self.payload.revision,
            "correlationId": self.correlation_id.to_string(),
        };
        if let Some(traceparent) = &self.traceparent {
            receipt.insert("traceparent", traceparent);
        }
        receipt
    }
}

fn valid_traceparent(value: &str) -> bool {
    let mut fields = value.split('-');
    let hex = |field: Option<&str>, len, nonzero: bool| {
        field.is_some_and(|field| {
            field.len() == len
                && field
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && (!nonzero || field.bytes().any(|b| b != b'0'))
        })
    };
    fields.next() == Some("00")
        && hex(fields.next(), 32, true)
        && hex(fields.next(), 16, true)
        && hex(fields.next(), 2, false)
        && fields.next().is_none()
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let (role, sequence) = match args.as_slice() {
        [role] if matches!(role.as_str(), "init" | "relay" | "consumer") => (role, None),
        [role, sequence] if role == "replay" => {
            let sequence = sequence
                .parse::<u64>()
                .ok()
                .filter(|&n| n > 0)
                .ok_or_else(|| io::Error::other("Expected a positive stream sequence"))?;
            (role, Some(sequence))
        }
        _ => {
            return Err(io::Error::other(
                "Expected init, relay, consumer, or replay <sequence>",
            ));
        }
    };
    run(role, sequence).await.map_err(|error| {
        if let Some(error) = error.downcast_ref::<ReplayDenied>() {
            return io::Error::other(error.0);
        }
        // Driver diagnostics may contain credentials, subjects, or document data.
        io::Error::other(format!(
            "{role} stopped: messaging or persistence unavailable"
        ))
    })
}

async fn run(role: &str, sequence: Option<u64>) -> Result<()> {
    let options = async_nats::ConnectOptions::with_user_and_password(
        env::var("NATS_USER")?,
        env::var("NATS_PASSWORD")?,
    )
    .connection_timeout(Duration::from_secs(3));
    let options = if sequence.is_none() {
        options.retry_on_initial_connect()
    } else {
        options
    };
    let nats = options.connect("nats://nats:4222").await?;
    let mut js = jetstream::new(nats);
    js.set_timeout(Duration::from_secs(5));
    if role == "init" {
        return provision(&js).await;
    }
    let mongo = Client::with_uri_str(env::var("MONGODB_URI")?).await?;
    if let Some(sequence) = sequence {
        return replay(&js, &mongo.database("article_receipts"), sequence).await;
    }
    match role {
        "relay" => relay(&js, mongo.database("master_data").collection("outbox")).await,
        _ => consume(&js, mongo.database("article_receipts")).await,
    }
}

#[derive(Debug)]
struct ReplayDenied(&'static str);

impl std::fmt::Display for ReplayDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ReplayDenied {}

async fn replay(js: &Context, db: &mongodb::Database, sequence: u64) -> Result<()> {
    let key = sequence.to_string();
    let entries = db.collection::<Document>("quarantine");
    let entry = entries
        .find_one(doc! {"_id": &key, "reason": "receipt delivery limit reached"})
        .await?
        .ok_or(ReplayDenied(
            "No retry-exhausted quarantine entry for that sequence",
        ))?;
    if entry.contains_key("replayedAt") {
        return Err(ReplayDenied("This quarantine entry has already been replayed").into());
    }
    let original = js
        .get_stream_no_info(STREAM)
        .await?
        .get_raw_message(sequence)
        .await?;
    let event: Event = serde_json::from_slice(&original.payload)?;
    if original.subject.as_str() != SUBJECT
        || !event.valid()
        || entry.get_str("messageId")? != event.message_id.to_string()
        || entry.get_str("tenantId")? != event.tenant_id.to_string()
        || entry.get_str("organizationId")? != event.organization_id.to_string()
        || entry.get_str("correlationId")? != event.correlation_id.to_string()
    {
        return Err(ReplayDenied("Stored message does not match the quarantine entry").into());
    }
    let requested = entries
        .update_one(
            doc! {"_id": &key, "replayedAt": {"$exists": false}},
            doc! {"$currentDate": {"replayRequestedAt": true}},
        )
        .await?;
    if requested.matched_count != 1 {
        return Err(ReplayDenied("Quarantine entry changed; replay not sent").into());
    }
    let mut headers = async_nats::HeaderMap::new();
    headers.insert(
        "Nats-Msg-Id",
        format!("replay-{sequence}-{}", event.message_id),
    );
    if let Some(traceparent) = event.traceparent.as_deref() {
        headers.insert("traceparent", traceparent);
    }
    let ack = js
        .publish_with_headers(SUBJECT, headers, original.payload)
        .await?
        .await?;
    if ack.stream != STREAM {
        return Err(io::Error::other("Unexpected replay stream").into());
    }
    entries
        .update_one(
            doc! {"_id": &key},
            doc! {
                "$currentDate": {"replayedAt": true},
                "$set": {"replaySequence": ack.sequence.to_string()},
            },
        )
        .await?;
    println!("Replay accepted for stream sequence {sequence}");
    Ok(())
}
async fn provision(js: &Context) -> Result<()> {
    let stream = js
        .get_or_create_stream(stream::Config {
            name: STREAM.into(),
            subjects: vec![SUBJECT.into()],
            max_bytes: 32 * 1024 * 1024,
            discard: stream::DiscardPolicy::New,
            duplicate_window: Duration::from_secs(2),
            ..Default::default()
        })
        .await?;
    stream
        .get_or_create_consumer(
            CONSUMER,
            consumer::pull::Config {
                durable_name: Some(CONSUMER.into()),
                filter_subject: SUBJECT.into(),
                ack_policy: consumer::AckPolicy::Explicit,
                ack_wait: Duration::from_secs(15),
                max_ack_pending: 1,
                ..Default::default()
            },
        )
        .await?;
    Ok(())
}

async fn relay(js: &Context, outbox: Collection<Document>) -> Result<()> {
    loop {
        match publish_next(js, &outbox).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(_) => eprintln!("Outbox delivery failed; will retry"),
        }
        tokio::time::sleep(PAUSE).await;
    }
}

async fn publish_next(js: &Context, outbox: &Collection<Document>) -> Result<bool> {
    let Some(mut envelope) = outbox
        .find_one(doc! {"publishedAt": {"$exists": false}})
        .sort(doc! {"timestamp": 1, "_id": 1})
        .await?
    else {
        return Ok(false);
    };
    let id = envelope
        .remove("_id")
        .ok_or_else(|| io::Error::other("Missing outbox key"))?;
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Nats-Msg-Id", envelope.get_str("messageId")?);
    if let Some(traceparent) = envelope
        .get_str("traceparent")
        .ok()
        .filter(|value| valid_traceparent(value))
    {
        headers.insert("traceparent", traceparent);
    }
    js.publish_with_headers(SUBJECT, headers, serde_json::to_vec(&envelope)?.into())
        .await?
        .await?;
    outbox
        .update_one(
            doc! {"_id": id},
            doc! {"$currentDate": {"publishedAt": true}},
        )
        .await?;
    Ok(true)
}

async fn consume(js: &Context, db: mongodb::Database) -> Result<()> {
    let stream = js.get_stream_no_info(STREAM).await?;
    let consumer: consumer::PullConsumer = stream.get_consumer(CONSUMER).await?;
    let mut messages = consumer
        .stream()
        .max_messages_per_batch(1)
        .messages()
        .await?;
    let receipts = db.collection::<Document>("receipts");
    let quarantine = db.collection::<Document>("quarantine");
    while let Some(message) = messages.next().await {
        let message = message?;
        let info = message.info()?;
        let entry = match serde_json::from_slice::<Event>(&message.payload) {
            Ok(event) if event.valid() => {
                if info.delivered <= MAX_RECEIPT_DELIVERIES
                    && persist(&receipts, event.message_id.to_string(), event.receipt())
                        .await
                        .is_ok()
                {
                    message.double_ack().await?;
                    continue;
                }
                if let Some(delay) = receipt_retry_delay(info.delivered) {
                    eprintln!("Article receipt persistence failed; requesting redelivery");
                    message.ack_with(AckKind::Nak(Some(delay))).await?;
                    continue;
                }
                let mut entry = doc! {
                    "reason": "receipt delivery limit reached",
                    "messageId": event.message_id.to_string(),
                    "correlationId": event.correlation_id.to_string(),
                    "tenantId": event.tenant_id.to_string(),
                    "organizationId": event.organization_id.to_string(),
                    "deliveryCount": info.delivered,
                };
                if let Some(traceparent) = &event.traceparent {
                    entry.insert("traceparent", traceparent);
                }
                entry
            }
            _ => doc! {"reason": "invalid ArticleRegistered envelope"},
        };
        if persist(&quarantine, info.stream_sequence.to_string(), entry)
            .await
            .is_ok()
        {
            message.ack_with(AckKind::Term).await?;
        } else {
            // Do not let a broker-side delivery cap discard an unrecorded message.
            eprintln!("Article quarantine persistence failed; requesting redelivery");
            message
                .ack_with(AckKind::Nak(Some(Duration::from_secs(10))))
                .await?;
        }
    }
    Err(io::Error::other("Consumer message stream ended").into())
}

fn receipt_retry_delay(delivered: i64) -> Option<Duration> {
    let index = usize::try_from(delivered.checked_sub(1)?).ok()?;
    RECEIPT_RETRY_DELAYS.get(index).copied()
}

async fn persist(collection: &Collection<Document>, key: String, data: Document) -> Result<()> {
    collection
        .update_one(doc! {"_id": key}, doc! {"$setOnInsert": data})
        .upsert(true)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::valid_traceparent;

    #[test]
    fn accepts_only_nonzero_lowercase_v00_trace_context() {
        assert!(valid_traceparent(
            "00-123456789abcdef0123456789abcdef0-123456789abcdef0-00"
        ));
        for invalid in [
            "00-00000000000000000000000000000000-123456789abcdef0-00",
            "00-123456789abcdef0123456789abcdef0-0000000000000000-00",
            "00-123456789abcdef0123456789abcdef0-123456789ABCDEF0-00",
            "00-123456789abcdef0123456789abcdef0-123456789abcdef0-00-extra",
        ] {
            assert!(!valid_traceparent(invalid), "{invalid}");
        }
    }
}
