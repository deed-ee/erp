use super::*;
use crate::{receipt_wire, receipt_wire::tests::fixture};
use erp4us_messaging::HeaderMap;
use mongodb::Client;
use serde_json::{Value, json};

struct Sample {
    scope: Scope,
    subject: String,
    headers: HeaderMap,
    event: Value,
}

impl Sample {
    fn new() -> Self {
        let (scope, subject, headers, event) = fixture();
        Self {
            scope,
            subject,
            headers,
            event,
        }
    }

    fn line(&self, event: &Value) -> LineKey<'static> {
        let id = |key: &str| serde_json::from_value(event["payload"][key].clone()).unwrap();
        LineKey {
            tenant_id: self.scope.tenant_id,
            organization_id: self.scope.organization_id,
            plan_id: id("planId"),
            purchase_order_id: id("purchaseOrderId"),
            purchase_line_id: id("purchaseLineId"),
            sku: "NM-ENDPLATE",
            revision: None,
            unit: "EA",
        }
    }

    fn lot(&self, sequence: u32, pieces: &str, identity: u128) -> Value {
        let mut event = self.event.clone();
        let message = Uuid::from_u128(1_000 + identity);
        event["messageId"] = json!(message);
        event["correlationId"] = json!(message);
        event["payload"]["receiptId"] = json!(Uuid::from_u128(2_000 + identity));
        event["payload"]["receiptSequence"] = json!(sequence);
        event["payload"]["pieces"] = json!(pieces);
        event
    }

    fn decode(&self, value: &Value, replay: bool) -> Envelope<ReceiptPosted> {
        let mut headers = self.headers.clone();
        let message = value["messageId"].as_str().unwrap();
        headers.insert("Erp4us-Message-Id", message);
        headers.insert("Erp4us-Correlation-Id", message);
        headers.insert(
            "Erp4us-Organization-Id",
            value["organizationId"].as_str().unwrap(),
        );
        let scope = Scope {
            organization_id: serde_json::from_value(value["organizationId"].clone()).unwrap(),
            ..self.scope
        };
        let body = if replay {
            headers.insert("Nats-Msg-Id", format!("replay-42-{message}").as_str());
            serde_json::to_vec_pretty(value).unwrap()
        } else {
            headers.insert("Nats-Msg-Id", message);
            serde_json::to_vec(value).unwrap()
        };
        receipt_wire::decode(scope, &self.subject, &headers, &body).unwrap()
    }
}

fn observed(received: u32, open: u32, through: u32) -> Observation {
    Observation::Observed {
        received_pieces: received,
        open_pieces: open,
        through_receipt_sequence: through,
    }
}

#[test]
fn private_inbox_round_trips_scoped_normalized_identity() {
    let sample = Sample::new();
    let event = sample.decode(&sample.event, false);
    let prepared = Prepared::decoded(sample.scope, &event).unwrap();
    let doc = bson::to_document(&prepared.inbox).unwrap();
    assert_eq!(
        doc.get_document("_id").unwrap(),
        message_filter(sample.scope, event.message_id)
            .get_document("_id")
            .unwrap(),
    );
    let line = LineId::from(sample.line(&sample.event));
    assert_eq!(doc.get_document("line"), Ok(&line.key()));
    assert_eq!(
        doc.get_document("pin").unwrap().get("revision"),
        Some(&bson::Bson::Null)
    );
    let fields = [
        "_id",
        "schemaVersion",
        "line",
        "pin",
        "receiptId",
        "sequence",
        "pieces",
        "owner",
    ];
    assert!(fields.into_iter().all(|field| doc.contains_key(field)));
    assert_eq!(doc.len(), fields.len());
    let rec: Inbox = bson::from_slice(&bson::to_vec(&doc).unwrap()).unwrap();
    let restored = Prepared::stored(rec).unwrap();
    let wire_replay = Prepared::decoded(sample.scope, &sample.decode(&sample.event, true)).unwrap();
    assert!(restored.fact().unwrap() == wire_replay.fact().unwrap());

    let mut changed = sample.event.clone();
    changed["timestamp"] = json!("2026-11-28T09:01:00Z");
    let changed = Prepared::decoded(sample.scope, &sample.decode(&changed, false)).unwrap();
    assert!(restored.fact().unwrap() != changed.fact().unwrap());

    let mut corrupt = doc;
    corrupt
        .get_document_mut("line")
        .unwrap()
        .insert("organizationId", Uuid::new_v4().to_string());
    let rec: Inbox = bson::from_document(corrupt).unwrap();
    assert_eq!(Prepared::stored(rec).err(), Some(Failure::Unavailable));
}

#[test]
fn issued_scope_and_persisted_summary_must_agree() {
    let sample = Sample::new();
    let key = sample.line(&sample.event);
    for foreign in [
        Scope {
            tenant_id: Uuid::new_v4(),
            ..sample.scope
        },
        Scope {
            organization_id: Uuid::new_v4(),
            ..sample.scope
        },
    ] {
        assert_eq!(
            IssuedLine::new(foreign, key, "12").err(),
            Some(Failure::InvalidLine)
        );
        assert_eq!(
            Prepared::decoded(foreign, &sample.decode(&sample.event, false)).err(),
            Some(Failure::InvalidFact)
        );
    }
    let issued = IssuedLine::new(sample.scope, key, "12").unwrap();
    let summary = Summary::initial(issued);
    assert_eq!(
        summary.projection(issued).unwrap().observation(),
        Observation::Unknown
    );
    let altered = IssuedLine::new(sample.scope, key, "13").unwrap();
    assert_eq!(
        summary.projection(altered).err(),
        Some(Failure::Unavailable)
    );
    let mut malformed = bson::to_document(&summary).unwrap();
    malformed.insert("through", 3);
    let malformed: Summary = bson::from_document(malformed).unwrap();
    assert_eq!(
        malformed.projection(issued).err(),
        Some(Failure::Unavailable)
    );
}

#[tokio::test]
#[ignore = "requires PROCUREMENT_RECEIPT_TEST_MONGO_URI for a dedicated disposable replica set"]
async fn mongo_persists_gaps_replay_conflicts_and_bounded_continuation() {
    let uri = std::env::var("PROCUREMENT_RECEIPT_TEST_MONGO_URI").expect("dedicated Mongo URI");
    let name = format!("procurement_receipt_test_{}", Uuid::new_v4().simple());
    let sample = Sample::new();
    let db = Client::with_uri_str(&uri).await.unwrap().database(&name);
    let mut store = Store::open(&db).await.unwrap();
    let key = sample.line(&sample.event);
    let issued = IssuedLine::new(sample.scope, key, "12").unwrap();
    let unknown = View {
        state: Observation::Unknown,
        projected_at: None,
    };
    assert_eq!(store.read(issued).await.unwrap(), unknown);

    let mut unrelated = sample.lot(1, "6", 1);
    unrelated["payload"]["purchaseOrderId"] = json!(Uuid::new_v4());
    for event in [&unrelated, &sample.event] {
        let scope = if event == &unrelated {
            sample.scope
        } else {
            Scope {
                organization_id: Uuid::new_v4(),
                ..sample.scope
            }
        };
        let mut event = event.clone();
        event["organizationId"] = json!(scope.organization_id);
        assert_eq!(
            store
                .record(issued, scope, &sample.decode(&event, false))
                .await
                .unwrap(),
            Admission::NotForLine
        );
    }
    let inbox = db.collection::<Document>(INBOX);
    let summaries = db.collection::<Document>(SUMMARIES);
    assert_eq!(inbox.count_documents(doc! {}).await.unwrap(), 0);
    assert_eq!(summaries.count_documents(doc! {}).await.unwrap(), 0);

    let second = sample.lot(2, "6", 2);
    assert_eq!(
        store
            .record(issued, sample.scope, &sample.decode(&second, false))
            .await
            .unwrap(),
        Admission::Staged
    );
    assert_eq!(store.read(issued).await.unwrap(), unknown);
    let summary_filter = doc! {"_id": LineId::from(key).key()};
    let before = summaries
        .find_one(summary_filter.clone())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(before.get_bool("pending"), Ok(false)); // Known gap, not runnable.

    // New client/store, same independent persisted state; pretty-printed
    // wire and operator replay key must still be the same normalized fact.
    drop(store);
    let reopened = Client::with_uri_str(&uri).await.unwrap().database(&name);
    store = Store::open(&reopened).await.unwrap();
    assert_eq!(
        store
            .record(issued, sample.scope, &sample.decode(&second, true))
            .await
            .unwrap(),
        Admission::Replay
    );
    assert_eq!(
        summaries.find_one(summary_filter.clone()).await.unwrap(),
        Some(before)
    );
    assert_eq!(store.read(issued).await.unwrap(), unknown);
    let first = sample.decode(&sample.event, false);
    let replay = sample.decode(&sample.event, true);
    let (a, b) = tokio::join!(
        store.record(issued, sample.scope, &first),
        store.record(issued, sample.scope, &replay),
    );
    assert!(matches!(
        (a.unwrap(), b.unwrap()),
        (Admission::Staged, Admission::Replay) | (Admission::Replay, Admission::Staged)
    ));
    let complete = store.read(issued).await.unwrap();
    assert_eq!(complete.state, observed(12, 0, 2));
    assert!(complete.projected_at.is_some());
    assert_eq!(
        store
            .record(issued, sample.scope, &sample.decode(&sample.event, true))
            .await
            .unwrap(),
        Admission::Replay
    );
    assert_eq!(store.read(issued).await.unwrap(), complete);
    assert_eq!(inbox.count_documents(doc! {}).await.unwrap(), 2);

    let over = sample.lot(3, "1", 3);
    assert_eq!(
        store
            .record(issued, sample.scope, &sample.decode(&over, false))
            .await
            .unwrap(),
        Admission::Conflict(Conflict::OverOrdered)
    );
    assert_eq!(store.read(issued).await.unwrap(), unknown);
    let quarantined = summaries.find_one(summary_filter).await.unwrap().unwrap();
    assert_eq!(quarantined.get_i64("through"), Ok(2));
    assert_eq!(quarantined.get_i64("received"), Ok(12));
    assert_eq!(
        quarantined
            .get_document("quarantine")
            .unwrap()
            .get_str("reason"),
        Ok("over-ordered")
    );
    assert_eq!(inbox.count_documents(doc! {}).await.unwrap(), 2);

    let other = Uuid::new_v4();
    let other_key = LineKey {
        purchase_line_id: other,
        ..key
    };
    let other_line = IssuedLine::new(sample.scope, other_key, "12").unwrap();
    let other_summary = doc! {"_id": LineId::from(other_key).key()};
    let lot = |sequence| {
        let mut event = sample.lot(sequence, "1", 100 + u128::from(sequence));
        event["payload"]["purchaseLineId"] = json!(other);
        event
    };
    let last = MAX_DRAIN as u32 + 2;
    for sequence in 2..=last {
        assert_eq!(
            store
                .record(
                    other_line,
                    sample.scope,
                    &sample.decode(&lot(sequence), false)
                )
                .await
                .unwrap(),
            Admission::Staged
        );
    }
    assert_eq!(
        store
            .record(other_line, sample.scope, &sample.decode(&lot(1), false))
            .await
            .unwrap(),
        Admission::Staged
    );
    let pending = summaries
        .find_one(other_summary.clone())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending.get_bool("pending"), Ok(true));
    assert_eq!(pending.get_i64("through"), Ok(MAX_DRAIN as i64));
    assert_eq!(store.read(other_line).await.unwrap(), unknown);
    drop(store);
    store = Store::open(&db).await.unwrap();
    store.continue_pending(other_line).await.unwrap();
    assert_eq!(
        store.read(other_line).await.unwrap().state,
        observed(last, 12 - last, last)
    );

    // Old and new tuples really are authorized issued lines; only these
    // two verified siblings are linked by the reused immutable message ID.
    let mut collision = lot(1);
    let third_key = LineKey {
        purchase_line_id: Uuid::new_v4(),
        ..key
    };
    collision["payload"]["purchaseLineId"] = json!(third_key.purchase_line_id);
    let third_line = IssuedLine::new(sample.scope, third_key, "12").unwrap();
    assert_eq!(
        store
            .record(third_line, sample.scope, &sample.decode(&collision, false))
            .await
            .unwrap(),
        Admission::Conflict(Conflict::Message)
    );
    assert_eq!(store.read(third_line).await.unwrap(), unknown);
    assert_eq!(store.read(other_line).await.unwrap(), unknown);
    let blocked = summaries.find_one(other_summary).await.unwrap().unwrap();
    assert_eq!(blocked.get_i64("through"), Ok(i64::from(last)));
    assert_eq!(blocked.get_i64("received"), Ok(i64::from(last)));
    assert_eq!(
        inbox.count_documents(doc! {}).await.unwrap(),
        u64::from(last + 2)
    );
    db.drop().await.unwrap();
}
