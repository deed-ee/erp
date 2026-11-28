//! Test-only delivery of an exact committed receipt. No runtime relay or broker grants.

use super::{COMMIT_LIMIT, DB_LIMIT, Organization, RECEIPTS, Store, fact, majority_write, records};
use erp4us_messaging::{Envelope, JetStream};
use mongodb::bson::{DateTime, Document, doc};
use tokio::time::timeout;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Failure {
    Storage,
    InvalidFact,
    NotAcknowledged,
}

pub(super) fn decode(
    scope: Organization,
    receipt: Uuid,
    row: &Document,
) -> Result<Envelope<fact::Posted>, Failure> {
    let invalid = |_| Failure::InvalidFact;
    let actor = row.get_str("actorId").map_err(invalid)?;
    let actor = Uuid::try_parse(actor).map_err(|_| Failure::InvalidFact)?;
    let intent =
        records::read_intent(scope, actor, receipt, row).map_err(|_| Failure::InvalidFact)?;
    let saved = intent.confirmed.ok_or(Failure::InvalidFact)?;
    let outbox = row.get_document("outbox").map_err(invalid)?;
    if outbox.get_bool("pending") != Ok(true) {
        return Err(Failure::InvalidFact);
    }
    let body = outbox.get_str("envelope").map_err(invalid)?;
    if body.len() > 7 * 1024 {
        return Err(Failure::InvalidFact);
    }
    let event: Envelope<fact::Posted> =
        serde_json::from_str(body).map_err(|_| Failure::InvalidFact)?;
    event.validate().map_err(|_| Failure::InvalidFact)?;
    if event.source_service != fact::SOURCE
        || event.message_id != saved.message_id
        || event.correlation_id != saved.message_id
        || event.causation_id.is_some()
        || event.timestamp != fact::timestamp(&saved).map_err(|_| Failure::InvalidFact)?
        || event.tenant_id != Some(scope.tenant)
        || event.organization_id != Some(scope.organization)
        || !event
            .actor
            .as_ref()
            .is_some_and(|recorded| recorded.id == actor.to_string())
        || event.payload != fact::Posted::from_receipt(&saved)
        || serde_json::to_string(&event).map_err(|_| Failure::InvalidFact)? != body
    {
        return Err(Failure::InvalidFact);
    }
    Ok(event)
}

impl Store {
    /// The caller supplies a fixture's pre-authorized scope; a receipt ID is not
    /// authority. Only a broker-acknowledged event advances this outbox cursor.
    pub(super) async fn publish_pending_receipt(
        &self,
        scope: Organization,
        receipt: Uuid,
        jetstream: &JetStream,
    ) -> Result<Option<(u64, bool)>, Failure> {
        let identity = records::receipt_ref(scope, receipt);
        let pending = self.majority_read(RECEIPTS);
        let row = timeout(
            DB_LIMIT,
            pending
                .find_one(doc! {"_id": &identity, "state": "confirmed", "outbox.pending": true})
                .max_time(DB_LIMIT)
                .into_future(),
        )
        .await
        .map_err(|_| Failure::Storage)?
        .map_err(|_| Failure::Storage)?;
        let Some(row) = row else { return Ok(None) };
        let event = decode(scope, receipt, &row)?;
        let ack = timeout(COMMIT_LIMIT, jetstream.publish(&event))
            .await
            .map_err(|_| Failure::NotAcknowledged)?
            .map_err(|_| Failure::NotAcknowledged)?;
        timeout(
            DB_LIMIT,
            self.db
                .collection::<Document>(RECEIPTS)
                .update_one(
                    doc! {
                        "_id": identity, "state": "confirmed", "outbox.pending": true,
                        "outbox.messageId": event.message_id.to_string(),
                    },
                    doc! {"$set": {"outbox.pending": false, "outbox.publishedAt": DateTime::now()}},
                )
                .write_concern(majority_write())
                .into_future(),
        )
        .await
        .map_err(|_| Failure::Storage)?
        .map_err(|_| Failure::Storage)?;
        Ok(Some((ack.sequence, ack.duplicate)))
    }
}
