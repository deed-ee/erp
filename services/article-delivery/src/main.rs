use erp4us_messaging::{
    AckKind, ConnectOptions, Connection, Contract, Delivery, Envelope, Error, JetStream, Kind,
    Layer, Route, Scope, consumer, stream,
};
use futures_util::StreamExt;
use mongodb::{
    Client, Collection,
    bson::{Document, doc},
};
use serde::{Deserialize, Serialize};
use std::{
    env, io,
    time::{Duration, Instant},
};
use uuid::{Uuid, uuid};

const LEGACY_SUBJECT: &str = "business.master-data.article.registered.v1";
const STREAM: &str = "ERP4US_BUSINESS_EVENTS_V1";
const LOCAL_TENANT: Uuid = uuid!("ddcae139-c1e8-47d2-8b48-349cb6f30ce5");
const CONSUMER: &str = "article-receipts";
const PAUSE: Duration = Duration::from_secs(1);
const RECEIPT_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];
const MAX_RECEIPT_DELIVERIES: i64 = 4;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    article_id: Uuid,
    revision: u32,
}

impl Contract for Payload {
    const ROUTE: Route<'static> = Route {
        layer: Layer::Business,
        domain: "master-data",
        kind: Kind::Event,
        resource: "article",
        action: "registered",
    };
    const SCHEMA_VERSION: &'static str = "1.0.0";
    const PROTOCOL_SUPPORT: &'static str = "=1.0.0";
    const SCHEMA_SUPPORT: &'static str = "=1.0.0";
    const DELIVERY: Delivery = Delivery::JetStream(STREAM);

    fn validate(&self) -> std::result::Result<(), Error> {
        if self.article_id.is_nil() || self.revision != 1 {
            return Err(Error::InvalidEnvelope(
                "ArticleRegistered requires an article ID and revision 1",
            ));
        }
        Ok(())
    }
}

struct Event {
    envelope: Envelope<Payload>,
    tenant: Uuid,
    organization: Uuid,
}

impl Event {
    fn new(envelope: Envelope<Payload>) -> std::result::Result<Self, Error> {
        let (Some(tenant), Some(organization)) = (envelope.tenant_id, envelope.organization_id)
        else {
            return Err(Error::InvalidEnvelope(
                "ArticleRegistered requires tenant and organization",
            ));
        };
        if envelope.source_service != "erp4us.master-data" {
            return Err(Error::InvalidEnvelope("ArticleRegistered source"));
        }
        Ok(Self {
            envelope,
            tenant,
            organization,
        })
    }

    fn receipt(&self) -> Document {
        let event = &self.envelope;
        let mut receipt = doc! {
            "articleId": event.payload.article_id.to_string(),
            "tenantId": self.tenant.to_string(),
            "organizationId": self.organization.to_string(),
            "revision": event.payload.revision,
            "correlationId": event.correlation_id.to_string(),
        };
        if let Some(traceparent) = &event.traceparent {
            receipt.insert("traceparent", traceparent);
        }
        receipt
    }
}

fn from_outbox(mut document: Document) -> Result<Event> {
    // The database format is intentionally stable; only the unpublished wire projection changes.
    if document.get_str("messageType")? != LEGACY_SUBJECT || document.get_i32("schemaVersion")? != 1
    {
        return Err(Error::InvalidEnvelope("unrecognized Article outbox format").into());
    }
    let tenant = Uuid::parse_str(document.get_str("tenantId")?)?;
    document.insert(
        "messageType",
        Payload::ROUTE.subject(1, Scope::Tenant(tenant), 1)?,
    );
    document.insert("protocolVersion", "1.0.0");
    document.insert("schemaVersion", Payload::SCHEMA_VERSION);
    document.insert("serviceVersion", env!("CARGO_PKG_VERSION"));
    let event = serde_json::from_value::<Envelope<Payload>>(serde_json::to_value(document)?)?;
    event.validate()?;
    Ok(Event::new(event)?)
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
    let options =
        ConnectOptions::with_user_and_password(env::var("NATS_USER")?, env::var("NATS_PASSWORD")?)
            .custom_inbox_prefix(format!("_INBOX.article-{role}"))
            .connection_timeout(Duration::from_secs(3));
    let options = if sequence.is_none() {
        options.retry_on_initial_connect()
    } else {
        options
    };
    let url = env::var("NATS_URL").unwrap_or_else(|_| "nats://nats:4222".to_owned());
    let nats = Connection::connect(&url, options).await?;
    let js = nats.jetstream(Duration::from_secs(5));
    if role == "init" {
        return provision(&js).await;
    }
    let mongo = Client::with_uri_str(env::var("MONGODB_URI")?).await?;
    if let Some(sequence) = sequence {
        return replay(&js, &mongo.database("article_receipts"), sequence).await;
    }
    if role == "relay" {
        let seconds =
            env::var("HEARTBEAT_SECONDS").map_or(Ok(15), |seconds| seconds.parse::<u64>())?;
        let heartbeat =
            (seconds > 0).then(|| tokio::spawn(heartbeat(nats, Duration::from_secs(seconds))));
        let result = relay(&js, mongo.database("master_data").collection("outbox")).await;
        if let Some(heartbeat) = heartbeat {
            heartbeat.abort();
        }
        result
    } else {
        consume(&js, mongo.database("article_receipts")).await
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Heartbeat {
    instance_id: Uuid,
    uptime_seconds: u64,
    nats: String,
}

impl Contract for Heartbeat {
    const ROUTE: Route<'static> = Route {
        layer: Layer::Diagnostics,
        domain: "article-delivery",
        kind: Kind::Event,
        resource: "service",
        action: "heartbeat",
    };
    const SCHEMA_VERSION: &'static str = "1.0.0";
    const PROTOCOL_SUPPORT: &'static str = "=1.0.0";
    const SCHEMA_SUPPORT: &'static str = "=1.0.0";
    const DELIVERY: Delivery = Delivery::Core;
}

async fn heartbeat(nats: Connection, period: Duration) {
    let started = Instant::now();
    let instance_id = Uuid::new_v4();
    let mut tick = tokio::time::interval(period);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tick.tick().await;
        if nats.state() != erp4us_messaging::State::Connected {
            continue;
        }
        let payload = Heartbeat {
            instance_id,
            uptime_seconds: started.elapsed().as_secs(),
            nats: "connected".into(),
        };
        let result = Envelope::system(
            payload,
            erp4us_messaging::Version::new(1, 0, 0),
            "erp4us.article-delivery",
            erp4us_messaging::Version::new(0, 1, 0),
        );
        match result {
            Ok(envelope) if nats.publish(&envelope).await.is_ok() => {}
            _ => eprintln!("event=heartbeat_publish_failed service=article-delivery"),
        }
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

async fn replay(js: &JetStream, db: &mongodb::Database, sequence: u64) -> Result<()> {
    let key = quarantine_key(sequence);
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
        .operations()
        .get_stream_no_info(STREAM)
        .await?
        .get_raw_message(sequence)
        .await?;
    let event = Event::new(Envelope::decode(
        original.subject.as_str(),
        &original.headers,
        &original.payload,
    )?)?;
    if entry.get_str("messageId")? != event.envelope.message_id.to_string()
        || entry.get_str("tenantId")? != event.tenant.to_string()
        || entry.get_str("organizationId")? != event.organization.to_string()
        || entry.get_str("correlationId")? != event.envelope.correlation_id.to_string()
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
    let ack = js.replay::<Payload>(&original).await?;
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
async fn provision(js: &JetStream) -> Result<()> {
    let js = js.operations();
    match js.get_stream("ARTICLES").await {
        Ok(legacy) if legacy.cached_info().state.messages > 0 => {
            let consumer: consumer::PullConsumer = legacy.get_consumer(CONSUMER).await
                .map_err(|_| ReplayDenied("Legacy ARTICLES has messages but its durable could not be inspected; see NATS cutover runbook"))?;
            let info = consumer.cached_info();
            if info.num_pending != 0 || info.num_ack_pending != 0 {
                return Err(ReplayDenied(
                    "Legacy ARTICLES durable is not drained; see NATS cutover runbook",
                )
                .into());
            }
        }
        Ok(_) => {}
        Err(error)
            if matches!(error.kind(), erp4us_messaging::GetStreamErrorKind::JetStream(err)
            if err.error_code() == erp4us_messaging::ErrorCode::STREAM_NOT_FOUND) => {}
        Err(error) => return Err(error.into()),
    }
    let policies = [
        (STREAM, "v1.business.*.*.event.*.*.*", 32, 30),
        (
            "ERP4US_BUSINESS_COMMANDS_V1",
            "v1.business.*.*.command.*.*.*",
            8,
            7,
        ),
        (
            "ERP4US_CONFIG_EVENTS_V1",
            "v1.config.*.*.event.*.*.*",
            8,
            14,
        ),
        (
            "ERP4US_DIAGNOSTICS_CRITICAL_V1",
            "v1.diagnostics.*.*.event.*.critical.*",
            4,
            2,
        ),
    ];
    for (name, filter, mib, days) in policies {
        let config = stream::Config {
            name: name.into(),
            subjects: vec![filter.into()],
            max_bytes: mib * 1024 * 1024,
            max_age: Duration::from_secs(days * 86_400),
            discard: stream::DiscardPolicy::New,
            retention: stream::RetentionPolicy::Limits,
            storage: stream::StorageType::File,
            duplicate_window: Duration::from_secs(2),
            ..Default::default()
        };
        let live = js.get_or_create_stream(config.clone()).await?;
        let actual = &live.cached_info().config;
        if actual.subjects != config.subjects
            || actual.max_bytes != config.max_bytes
            || actual.max_age != config.max_age
            || actual.discard != config.discard
            || actual.retention != config.retention
            || actual.storage != config.storage
            || actual.duplicate_window != config.duplicate_window
        {
            return Err(ReplayDenied("Stream settings differ from the declared policy").into());
        }
    }
    let subject = Payload::ROUTE.subject(1, Scope::Tenant(LOCAL_TENANT), 1)?;
    let durable = js
        .get_stream(STREAM)
        .await?
        .get_or_create_consumer(
            CONSUMER,
            consumer::pull::Config {
                durable_name: Some(CONSUMER.into()),
                filter_subject: subject,
                ack_policy: consumer::AckPolicy::Explicit,
                ack_wait: Duration::from_secs(15),
                max_ack_pending: 1,
                ..Default::default()
            },
        )
        .await?;
    let actual = &durable.cached_info().config;
    if actual.filter_subject != Payload::ROUTE.subject(1, Scope::Tenant(LOCAL_TENANT), 1)?
        || !actual.filter_subjects.is_empty()
        || actual.ack_policy != consumer::AckPolicy::Explicit
        || actual.max_ack_pending != 1
        || actual.ack_wait != Duration::from_secs(15)
    {
        return Err(
            ReplayDenied("Article durable settings differ from the declared policy").into(),
        );
    }
    Ok(())
}

async fn relay(js: &JetStream, outbox: Collection<Document>) -> Result<()> {
    loop {
        match publish_next(js, &outbox).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(_) => eprintln!("Outbox delivery failed; will retry"),
        }
        tokio::time::sleep(PAUSE).await;
    }
}

async fn publish_next(js: &JetStream, outbox: &Collection<Document>) -> Result<bool> {
    let Some(mut envelope) = outbox
        .find_one(
            doc! {"publishedAt": {"$exists": false}, "deliveryRejectedAt": {"$exists": false}},
        )
        .sort(doc! {"timestamp": 1, "_id": 1})
        .await?
    else {
        return Ok(false);
    };
    let id = envelope
        .remove("_id")
        .ok_or_else(|| io::Error::other("Missing outbox key"))?;
    let event = match from_outbox(envelope) {
        Ok(event) => event,
        Err(_) => {
            eprintln!("Invalid Article outbox envelope; retaining rejected entry");
            outbox
                .update_one(
                    doc! {"_id": id},
                    doc! {"$currentDate": {"deliveryRejectedAt": true}},
                )
                .await?;
            return Ok(true);
        }
    };
    js.publish(&event.envelope).await?;
    outbox
        .update_one(
            doc! {"_id": id},
            doc! {"$currentDate": {"publishedAt": true}},
        )
        .await?;
    Ok(true)
}

async fn consume(js: &JetStream, db: mongodb::Database) -> Result<()> {
    let stream = js.operations().get_stream_no_info(STREAM).await?;
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
        let event = message
            .headers
            .as_ref()
            .ok_or(Error::InvalidHeader("Erp4us-Protocol-Version"))
            .and_then(|headers| {
                Envelope::decode(message.subject.as_str(), headers, &message.payload)
            })
            .and_then(Event::new);
        let entry = match event {
            Ok(event) => {
                if info.delivered <= MAX_RECEIPT_DELIVERIES
                    && persist(
                        &receipts,
                        event.envelope.message_id.to_string(),
                        event.receipt(),
                    )
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
                    "messageId": event.envelope.message_id.to_string(),
                    "correlationId": event.envelope.correlation_id.to_string(),
                    "tenantId": event.tenant.to_string(),
                    "organizationId": event.organization.to_string(),
                    "deliveryCount": info.delivered,
                };
                if let Some(traceparent) = &event.envelope.traceparent {
                    entry.insert("traceparent", traceparent);
                }
                entry
            }
            _ => doc! {"reason": "invalid ArticleRegistered envelope"},
        };
        if persist(&quarantine, quarantine_key(info.stream_sequence), entry)
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

fn quarantine_key(sequence: u64) -> String {
    format!("{STREAM}:{sequence}")
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
    use erp4us_messaging::valid_traceparent;

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
