use crate::Result;
use erp4us_messaging::{
    Connection, Contract, Delivery, Envelope, JetStream, Kind, Layer, Route, Scope, Version,
};
use futures_util::StreamExt;
use mongodb::{
    Collection, Database,
    bson::{Document, doc},
    error::{ErrorKind, WriteFailure},
};
use serde::{Deserialize, Serialize};
use std::io;
use tokio::sync::watch;

pub const STREAM: &str = "ERP4US_CONFIG_EVENTS_V1";

#[derive(Deserialize, Serialize)]
struct Get {}

#[derive(Deserialize, Serialize)]
struct Changed {
    revision: i64,
}

impl Contract for Get {
    const ROUTE: Route<'static> = route(Kind::Query, "get");
    const PROTOCOL_SUPPORT: &'static str = "=1.0.0";
    const SCHEMA_SUPPORT: &'static str = "=1.0.0";
    const SCHEMA_VERSION: &'static str = "1.0.0";
    const DELIVERY: Delivery = Delivery::Core;
}

impl Contract for Changed {
    const ROUTE: Route<'static> = route(Kind::Event, "changed");
    const PROTOCOL_SUPPORT: &'static str = "=1.0.0";
    const SCHEMA_SUPPORT: &'static str = "=1.0.0";
    const SCHEMA_VERSION: &'static str = "1.0.0";
    const DELIVERY: Delivery = Delivery::JetStream(STREAM);
}

const fn route(kind: Kind, action: &'static str) -> Route<'static> {
    Route {
        layer: Layer::Config,
        domain: "article-delivery",
        kind,
        resource: "heartbeat",
        action,
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    revision: i64,
    heartbeat_seconds: u64,
}

pub async fn read(nats: &Connection) -> Result<()> {
    let query = Envelope::system(
        Get {},
        Version::new(1, 0, 0),
        "erp4us.article-config-reader",
        Version::parse(env!("CARGO_PKG_VERSION"))?,
    )?;
    let response = nats
        .request(&query, std::time::Duration::from_secs(5))
        .await?;
    let response: serde_json::Value = serde_json::from_slice(&response.payload)?;
    let result = response
        .get("result")
        .filter(|_| response.get("error").is_none())
        .ok_or_else(|| io::Error::other("Heartbeat settings unavailable or invalid"))?;
    let snapshot: Snapshot = serde_json::from_value(result.clone())?;
    if snapshot.revision < 0 || snapshot.heartbeat_seconds > 3600 {
        return Err(io::Error::other("Invalid heartbeat settings snapshot").into());
    }
    println!("{}", serde_json::to_string(&snapshot)?);
    Ok(())
}

#[derive(Clone)]
pub struct Settings {
    journal: Collection<Document>,
    fallback: u64,
}

pub fn period(value: &str) -> io::Result<u64> {
    value
        .parse::<u64>()
        .ok()
        .filter(|seconds| *seconds <= 3600)
        .ok_or_else(|| {
            io::Error::other("Heartbeat period must be an integer from 0 to 3600 seconds")
        })
}

impl Settings {
    pub fn new(db: &Database, fallback: u64) -> Self {
        Self {
            journal: db.collection("article_relay_heartbeat_settings"),
            fallback,
        }
    }

    async fn snapshot(&self) -> Result<Snapshot> {
        let Some(row) = self
            .journal
            .find_one(doc! {})
            .sort(doc! {"_id": -1})
            .await?
        else {
            return Ok(Snapshot {
                revision: 0,
                heartbeat_seconds: self.fallback,
            });
        };
        Ok(Snapshot {
            revision: row.get_i64("_id")?,
            heartbeat_seconds: period(&row.get_i64("heartbeatSeconds")?.to_string())?,
        })
    }

    pub async fn set(&self, expected: i64, seconds: u64) -> Result<bool> {
        if self.snapshot().await?.revision != expected {
            return Ok(false);
        }
        let next = expected
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Revision exhausted"))?;
        let envelope = Envelope::system(
            Changed { revision: next },
            Version::new(1, 0, 0),
            "erp4us.article-delivery",
            Version::parse(env!("CARGO_PKG_VERSION"))?,
        )?;
        // The unique revision makes the append a compare-and-set and the row doubles as the outbox.
        match self
            .journal
            .insert_one(doc! {
                "_id": next,
                "heartbeatSeconds": i64::try_from(seconds)?,
                "notification": serde_json::to_string(&envelope)?,
            })
            .await
        {
            Ok(_) => Ok(true),
            Err(error)
                if matches!(&*error.kind, ErrorKind::Write(WriteFailure::WriteError(write))
                if write.code == 11000) =>
            {
                Ok(false)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub async fn serve(&self, nats: &Connection) -> Result<()> {
        let mut queries = nats.subscribe::<Get>(1, Scope::System, 1).await?;
        while let Some(message) = queries.next().await {
            let Some(query) = message
                .headers
                .as_ref()
                .and_then(|headers| {
                    Envelope::<Get>::decode(message.subject.as_str(), headers, &message.payload)
                        .ok()
                })
                .filter(|query| query.tenant_id.is_none())
            else {
                eprintln!("event=invalid_heartbeat_settings_query");
                continue;
            };
            let response = match self.snapshot().await {
                Ok(settings) => serde_json::json!({"result": settings}),
                Err(_) => {
                    eprintln!("event=heartbeat_settings_query_unavailable");
                    serde_json::json!({"error": {
                        "code": "unavailable", "retryable": true, "correlationId": query.correlation_id
                    }})
                }
            };
            if nats.reply(&message, &response).await.is_err() {
                eprintln!("event=heartbeat_settings_reply_failed");
            }
        }
        Err(io::Error::other("Heartbeat settings subscription ended").into())
    }

    pub async fn refresh(&self, js: &JetStream, period: watch::Sender<Option<u64>>) {
        let mut tick = tokio::time::interval(super::PAUSE);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut unavailable = false;
        loop {
            tick.tick().await;
            match self.snapshot().await {
                Ok(settings) => {
                    unavailable = false;
                    period.send_if_modified(|current| {
                        let new = Some(settings.heartbeat_seconds);
                        if *current == new {
                            return false;
                        }
                        *current = new;
                        true
                    });
                }
                Err(_) if !unavailable => {
                    unavailable = true;
                    eprintln!("event=heartbeat_settings_snapshot_unavailable");
                }
                Err(_) => {}
            }
            if self.publish_next(js).await.is_err() {
                eprintln!("event=heartbeat_settings_notification_failed");
            }
        }
    }

    async fn publish_next(&self, js: &JetStream) -> Result<()> {
        let Some(row) = self
            .journal
            .find_one(doc! {"publishedAt": {"$exists": false}})
            .sort(doc! {"_id": 1})
            .await?
        else {
            return Ok(());
        };
        let envelope: Envelope<Changed> = serde_json::from_str(row.get_str("notification")?)?;
        let revision = row.get_i64("_id")?;
        if revision <= 0 || envelope.payload.revision != revision {
            return Err(
                io::Error::other("Heartbeat settings notification revision mismatch").into(),
            );
        }
        js.publish(&envelope).await?;
        self.journal
            .update_one(
                doc! {"_id": revision, "publishedAt": {"$exists": false}},
                doc! {"$currentDate": {"publishedAt": true}},
            )
            .await?;
        Ok(())
    }
}
