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
    }

    fn receipt(&self) -> Document {
        doc! {
            "articleId": self.payload.article_id.to_string(),
            "tenantId": self.tenant_id.to_string(),
            "organizationId": self.organization_id.to_string(),
            "revision": self.payload.revision,
            "correlationId": self.correlation_id.to_string(),
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let role = env::args().nth(1).unwrap_or_default();
    if !matches!(role.as_str(), "init" | "relay" | "consumer") {
        return Err(io::Error::other("Expected init, relay, or consumer"));
    }
    run(&role).await.map_err(|_| {
        // Driver diagnostics may contain credentials, subjects, or document data.
        io::Error::other(format!(
            "{role} stopped: messaging or persistence unavailable"
        ))
    })
}

async fn run(role: &str) -> Result<()> {
    let options = async_nats::ConnectOptions::with_user_and_password(
        env::var("NATS_USER")?,
        env::var("NATS_PASSWORD")?,
    )
    .connection_timeout(Duration::from_secs(3))
    .retry_on_initial_connect();
    let nats = options.connect("nats://nats:4222").await?;
    let mut js = jetstream::new(nats);
    js.set_timeout(Duration::from_secs(5));
    if role == "init" {
        return provision(&js).await;
    }
    let mongo = Client::with_uri_str(env::var("MONGODB_URI")?).await?;
    match role {
        "relay" => relay(&js, mongo.database("master_data").collection("outbox")).await,
        _ => consume(&js, mongo.database("article_receipts")).await,
    }
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
        match serde_json::from_slice::<Event>(&message.payload) {
            Ok(event) if event.valid() => {
                if persist(&receipts, event.message_id.to_string(), event.receipt())
                    .await
                    .is_err()
                {
                    eprintln!("Article receipt persistence failed; requesting redelivery");
                    message.ack_with(AckKind::Nak(Some(PAUSE))).await?;
                } else {
                    message.double_ack().await?;
                }
            }
            _ => {
                persist(
                    &quarantine,
                    info.stream_sequence.to_string(),
                    doc! {"reason": "invalid ArticleRegistered envelope"},
                )
                .await?;
                message.ack_with(AckKind::Term).await?;
            }
        }
    }
    Err(io::Error::other("Consumer message stream ended").into())
}

async fn persist(collection: &Collection<Document>, key: String, data: Document) -> Result<()> {
    collection
        .update_one(doc! {"_id": key}, doc! {"$setOnInsert": data})
        .upsert(true)
        .await?;
    Ok(())
}
