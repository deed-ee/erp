use crate::{
    Scope, approval_store, budget_approvals, budget_policy, drafts,
    purchase_batch::{Batch, Intent, Issuance, IssueFailure, ReceiptSource},
    terms,
    terms_store::{self, DB_TIMEOUT, TransactionError, majority_collection, scoped_plan},
};
use mongodb::{
    ClientSession, Collection, Database,
    bson::{self, Document, doc},
    error::Error as MongoError,
};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;
use uuid::Uuid;

const BATCHES: &str = "issued_purchase_batches";

#[derive(Clone)]
pub(super) struct Store(Collection<Record>);

pub(super) struct Issue<'a> {
    pub drafts: &'a drafts::Store,
    pub terms: &'a terms_store::Store,
    pub approvals: &'a approval_store::Store,
    pub policy: Option<&'a budget_policy::Policy>,
    pub identity: erp4us_identity::Scope,
    pub plan_id: Uuid,
    pub intent: Intent,
}

pub(super) enum Saved {
    Created(Batch),
    Replay(Batch),
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    schema_version: i32,
    batch: Batch,
    approved_terms: terms::Saved,
    approval_decisions: Vec<budget_approvals::Record>,
}

fn key(scope: Scope, plan_id: Uuid) -> Document {
    doc! {"_id": scoped_plan(scope, plan_id)}
}

impl Record {
    fn check_plan(&self, plan_id: Uuid) -> Result<(), IssueFailure> {
        if self.schema_version != 1 || self.batch.plan_id() != plan_id {
            return Err(IssueFailure::Unavailable);
        }
        Ok(())
    }

    fn checked_batch(self, plan_id: Uuid) -> Result<Batch, IssueFailure> {
        self.check_plan(plan_id)?;
        Ok(self.batch)
    }

    fn receipt_source(
        &self,
        identity: erp4us_identity::Scope,
        plan_id: Uuid,
    ) -> Result<ReceiptSource, IssueFailure> {
        self.check_plan(plan_id)?;
        self.batch.receipt_source_for(
            identity.into(),
            &self.approved_terms,
            &self.approval_decisions,
            identity.actor,
        )
    }

    fn to_document(&self, scope: Scope) -> Result<Document, IssueFailure> {
        let mut document = key(scope, self.batch.plan_id());
        document.extend(bson::to_document(self).map_err(|_| IssueFailure::Unavailable)?);
        Ok(document)
    }
}

impl Store {
    pub(super) async fn open(db: &Database) -> std::io::Result<Self> {
        // Provision the immutable collection before any transaction uses it.
        // Its built-in unique _id enforces one batch per tenant/org/plan.
        terms_store::ensure_collection(db, BATCHES)
            .await
            .map_err(|_| std::io::Error::other("Cannot initialize Procurement purchase batches"))?;
        Ok(Self(
            db.collection_with_options(BATCHES, majority_collection()),
        ))
    }

    async fn find(&self, scope: Scope, plan_id: Uuid) -> Result<Record, IssueFailure> {
        self.0
            .find_one(key(scope, plan_id))
            .max_time(DB_TIMEOUT)
            .await
            .map_err(|_| IssueFailure::Unavailable)?
            .ok_or(IssueFailure::NotFound)
    }

    pub(super) async fn get(&self, scope: Scope, plan_id: Uuid) -> Result<Batch, IssueFailure> {
        self.find(scope, plan_id).await?.checked_batch(plan_id)
    }

    pub(super) async fn receipt_source(
        &self,
        identity: erp4us_identity::Scope,
        plan_id: Uuid,
    ) -> Result<ReceiptSource, IssueFailure> {
        self.find(identity.into(), plan_id)
            .await?
            .receipt_source(identity, plan_id)
    }

    pub(super) async fn exists(
        &self,
        scope: Scope,
        plan_id: Uuid,
        session: Option<&mut ClientSession>,
    ) -> Result<bool, MongoError> {
        let collection = self.0.clone_with_type::<Document>();
        let find = collection
            .find_one(key(scope, plan_id))
            .projection(doc! {"_id": 1})
            .max_time(DB_TIMEOUT);
        let batch = match session {
            Some(session) => find.session(session).await?,
            None => find.await?,
        };
        Ok(batch.is_some())
    }

    pub(super) async fn issue(
        &self,
        issue: Issue<'_>,
        deadline: Instant,
    ) -> Result<Saved, IssueFailure> {
        let scope = issue.identity.into();
        issue
            .terms
            .guard()
            .write(
                (scope, issue.plan_id),
                deadline,
                IssueFailure::Unavailable,
                &(self, issue),
                |(store, issue), session| Box::pin(store.apply(issue, session)),
            )
            .await
    }

    async fn apply(
        &self,
        issue: &Issue<'_>,
        session: &mut ClientSession,
    ) -> Result<Saved, TransactionError<IssueFailure>> {
        let scope = issue.identity.into();
        let plan_id = issue.plan_id;
        // Only the authorized original actor/command/intent can replay. Do not
        // re-form the batch or require the old policy to remain configured.
        if let Some(record) = self
            .0
            .find_one(key(scope, plan_id))
            .max_time(DB_TIMEOUT)
            .session(&mut *session)
            .await?
        {
            return record
                .checked_batch(plan_id)
                .and_then(|batch| batch.replay(issue.intent, issue.identity.actor))
                .map(Saved::Replay)
                .map_err(TransactionError::Rule);
        }
        let approved_terms = issue
            .terms
            .latest_in(scope, plan_id, session)
            .await?
            .ok_or(TransactionError::Rule(IssueFailure::NotFound))?;
        let attempt = Issuance::new(
            scope,
            &approved_terms,
            issue.policy,
            issue.intent,
            issue.identity.actor,
        )
        .map_err(TransactionError::Rule)?;
        let approval_decisions = issue
            .approvals
            .decisions(scope, plan_id, attempt.version(), session)
            .await?;
        let draft = issue
            .drafts
            .get_in(scope, plan_id, session)
            .await?
            .ok_or(TransactionError::Rule(IssueFailure::Unavailable))?;
        let batch = attempt
            .form(&draft, &approval_decisions)
            .map_err(TransactionError::Rule)?;
        let record = Record {
            schema_version: 1,
            batch,
            approved_terms,
            approval_decisions,
        };
        // The batch is also the command receipt. All assigned IDs, historical
        // values and the exact authorization/approval provenance commit together.
        let document = record.to_document(scope).map_err(TransactionError::Rule)?;
        self.0
            .clone_with_type::<Document>()
            .insert_one(document)
            .session(session)
            .await?;
        Ok(Saved::Created(record.batch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::purchase_batch::{
        IssuedBatch,
        tests::{ISSUER, approval_fixture},
    };
    use mongodb::bson::Bson;
    use serde_json::json;

    fn receipt_fixture() -> (Scope, Uuid, Document) {
        let (scope, draft, approved_terms, policy, intent, decisions) = approval_fixture();
        let plan = draft.plan.plan_id;
        let batch = Issuance::new(scope, &approved_terms, Some(&policy), intent, ISSUER)
            .unwrap()
            .form(&draft, &decisions)
            .unwrap();
        let record = Record {
            schema_version: 1,
            batch,
            approved_terms,
            approval_decisions: decisions.into(),
        };
        (scope, plan, record.to_document(scope).unwrap())
    }

    fn identity(scope: Scope, actor: Uuid) -> erp4us_identity::Scope {
        erp4us_identity::Scope {
            tenant: scope.tenant_id,
            organization: scope.organization_id,
            actor,
        }
    }

    fn decisions(document: &mut Document) -> &mut Vec<Bson> {
        document.get_array_mut("approvalDecisions").unwrap()
    }

    #[test]
    fn receipt_source_uses_only_scoped_issued_bson_and_denies_every_recorded_actor() {
        let (scope, plan, document) = receipt_fixture();
        assert_eq!(key(scope, plan).get("_id"), document.get("_id"));
        for foreign in [
            Scope {
                tenant_id: Uuid::new_v4(),
                ..scope
            },
            Scope {
                organization_id: Uuid::new_v4(),
                ..scope
            },
        ] {
            assert_ne!(key(foreign, plan).get("_id"), document.get("_id"));
        }
        let encoded = bson::to_vec(&document).unwrap();
        let issued = bson::from_slice::<Record>(&encoded).unwrap();
        for actor in [
            Uuid::nil(),
            issued.approved_terms.actor_id,
            issued.approval_decisions[0].actor_id(),
            issued.approval_decisions[1].actor_id(),
            ISSUER,
        ] {
            assert_eq!(
                issued
                    .receipt_source(identity(scope, actor), plan)
                    .unwrap_err(),
                IssueFailure::Forbidden
            );
        }

        // No configured policy, current terms, approval lookup, or projection
        // state is supplied to this read of the immutable issuance record.
        let receiver = identity(scope, Uuid::from_u128(11));
        let body = serde_json::to_vec(&issued.receipt_source(receiver, plan).unwrap()).unwrap();
        let batch = serde_json::to_value(IssuedBatch::try_from(issued.batch).unwrap()).unwrap();
        let mut expected: Vec<_> = batch["purchase_orders"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|order| {
                order["lines"].as_array().unwrap().iter().map(move |line| {
                    json!({
                        "purchase_order_id": order["purchase_order_id"],
                        "purchase_line_id": line["purchase_line_id"],
                        "sku": line["sku"],
                        "revision": line["revision"],
                        "unit": line["unit"],
                        "ordered_pieces": line["pieces"],
                    })
                })
            })
            .collect();
        expected.sort_unstable_by_key(|line| {
            (
                line["purchase_order_id"].to_string(),
                line["purchase_line_id"].to_string(),
            )
        });
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            json!({ "plan_id": plan, "lines": expected })
        );
        assert!(body.len() <= 16 * 1024);
    }

    #[test]
    fn receipt_source_fails_closed_on_incompatible_issuance_evidence() {
        let (scope, plan, document) = receipt_fixture();
        let receiver = identity(scope, Uuid::from_u128(11));
        let unavailable = |document| {
            let result = bson::from_document::<Record>(document)
                .map_err(|_| IssueFailure::Unavailable)
                .and_then(|issued| issued.receipt_source(receiver, plan));
            assert_eq!(result.unwrap_err(), IssueFailure::Unavailable);
        };

        let mut corrupt = document.clone();
        corrupt.insert("schemaVersion", 2);
        unavailable(corrupt);
        let mut corrupt = document.clone();
        decisions(&mut corrupt).pop();
        unavailable(corrupt);
        let mut corrupt = document.clone();
        decisions(&mut corrupt).swap(0, 1);
        unavailable(corrupt);
        let mut corrupt = document.clone();
        decisions(&mut corrupt).clear();
        unavailable(corrupt);
        let mut corrupt = document.clone();
        let records = decisions(&mut corrupt);
        let extra = records[1].clone();
        records.push(extra);
        unavailable(corrupt);
        let mut corrupt = document.clone();
        let records = decisions(&mut corrupt);
        let first_actor = records[0]
            .as_document()
            .unwrap()
            .get_str("actorId")
            .unwrap()
            .to_owned();
        records[1]
            .as_document_mut()
            .unwrap()
            .insert("actorId", first_actor);
        unavailable(corrupt);
        let mut corrupt = document.clone();
        decisions(&mut corrupt)[1]
            .as_document_mut()
            .unwrap()
            .get_document_mut("policySnapshot")
            .unwrap()
            .insert("require_distinct_approvers", false);
        unavailable(corrupt);
        let mut corrupt = document.clone();
        for record in decisions(&mut corrupt) {
            record
                .as_document_mut()
                .unwrap()
                .get_document_mut("policySnapshot")
                .unwrap()
                .insert("schema_version", 2);
        }
        unavailable(corrupt);
        let mut corrupt = document.clone();
        decisions(&mut corrupt)[1]
            .as_document_mut()
            .unwrap()
            .get_document_mut("originalResult")
            .unwrap()
            .insert("state", "pending");
        unavailable(corrupt);
        let mut corrupt = document.clone();
        corrupt
            .get_document_mut("approvedTerms")
            .unwrap()
            .insert("expectedRevision", 1);
        unavailable(corrupt);
        let mut corrupt = document.clone();
        corrupt
            .get_document_mut("approvedTerms")
            .unwrap()
            .get_document_mut("terms")
            .unwrap()
            .insert("plan_id", Uuid::new_v4().to_string());
        unavailable(corrupt);
        let mut corrupt = document.clone();
        corrupt
            .get_document_mut("batch")
            .unwrap()
            .get_document_mut("origin")
            .unwrap()
            .get_document_mut("intent")
            .unwrap()
            .insert("policy_revision", 2);
        unavailable(corrupt);
        let mut corrupt = document;
        corrupt
            .get_document_mut("batch")
            .unwrap()
            .get_array_mut("orders")
            .unwrap()[0]
            .as_document_mut()
            .unwrap()
            .get_array_mut("lines")
            .unwrap()[0]
            .as_document_mut()
            .unwrap()
            .insert("pieces", "1");
        unavailable(corrupt);
    }

    #[test]
    fn immutable_bson_batch_is_scoped_and_carries_exact_private_audit_and_replay() {
        let (scope, draft, terms, policy, intent, approvals) = approval_fixture();
        let plan_id = draft.plan.plan_id;
        let batch = Issuance::new(scope, &terms, Some(&policy), intent, ISSUER)
            .unwrap()
            .form(&draft, &approvals)
            .unwrap();
        let record = Record {
            schema_version: 1,
            batch,
            approved_terms: terms,
            approval_decisions: approvals.into(),
        };
        let doc = record.to_document(scope).unwrap();
        assert_eq!(
            doc.get_document("_id"),
            Ok(&doc! {
                "tenantId": scope.tenant_id.to_string(),
                "organizationId": scope.organization_id.to_string(),
                "planId": plan_id.to_string(),
            })
        );
        assert_ne!(key(scope, plan_id), key(scope, Uuid::new_v4()));
        for foreign in [
            Scope {
                tenant_id: Uuid::new_v4(),
                ..scope
            },
            Scope {
                organization_id: Uuid::new_v4(),
                ..scope
            },
        ] {
            assert_ne!(key(scope, plan_id), key(foreign, plan_id));
        }
        assert_eq!(doc.get_i32("schemaVersion"), Ok(1));
        let saved_batch = doc.get_document("batch").unwrap();
        let origin = saved_batch.get_document("origin").unwrap();
        assert_eq!(origin.get_str("actor_id").unwrap(), ISSUER.to_string());
        assert!(origin.get_datetime("issued_at").is_ok());
        assert_eq!(
            origin.get_document("intent").unwrap(),
            &bson::to_document(&intent).unwrap()
        );
        assert_eq!(saved_batch.get_str("total"), Ok("1882.40"));
        assert_eq!(saved_batch.get_array("orders").unwrap().len(), 2);
        let saved_terms = doc.get_document("approvedTerms").unwrap();
        assert_eq!(
            saved_terms.get_str("actorId").unwrap(),
            record.approved_terms.actor_id.to_string()
        );
        assert!(saved_terms.get_datetime("savedAt").is_ok());
        let approvals = doc.get_array("approvalDecisions").unwrap();
        assert_eq!(approvals.len(), 2);
        assert!(
            approvals[1]
                .as_document()
                .unwrap()
                .get_document("policySnapshot")
                .is_ok()
        );

        let encoded = bson::to_vec(&doc).unwrap();
        let restore = || bson::from_slice::<Record>(&encoded).unwrap();
        assert_eq!(restore(), record);
        let original = serde_json::to_value(IssuedBatch::try_from(record.batch).unwrap()).unwrap();
        // This is retrieved only from the stored receipt, even if policy is
        // removed or changed after issuance. No IDs/values are reassigned.
        let replay = restore()
            .checked_batch(plan_id)
            .unwrap()
            .replay(intent, ISSUER)
            .unwrap();
        assert_eq!(
            serde_json::to_value(IssuedBatch::try_from(replay).unwrap()).unwrap(),
            original
        );

        for (intent, actor) in [
            (intent, Uuid::from_u128(11)),
            (
                Intent {
                    command_id: Uuid::new_v4(),
                    ..intent
                },
                ISSUER,
            ),
            (
                Intent {
                    terms_revision: 2,
                    ..intent
                },
                ISSUER,
            ),
            (
                Intent {
                    policy_id: Uuid::new_v4(),
                    ..intent
                },
                ISSUER,
            ),
            (
                Intent {
                    policy_revision: 2,
                    ..intent
                },
                ISSUER,
            ),
        ] {
            assert_eq!(
                restore()
                    .checked_batch(plan_id)
                    .unwrap()
                    .replay(intent, actor),
                Err(IssueFailure::Conflict)
            );
        }
        assert_eq!(
            restore().checked_batch(Uuid::new_v4()),
            Err(IssueFailure::Unavailable)
        );
        let mut incompatible = restore();
        incompatible.schema_version = 2;
        assert_eq!(
            incompatible.checked_batch(plan_id),
            Err(IssueFailure::Unavailable)
        );
    }
}
