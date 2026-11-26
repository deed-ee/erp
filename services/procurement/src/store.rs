use crate::{Proposal, Scope, Shortage, Status};
use mongodb::{
    Collection, Database,
    bson::{self, DateTime, Document, doc},
    options::ReturnDocument,
};
use serde::Deserialize;
use uuid::Uuid;

pub(super) enum Failure {
    Unavailable,
    Conflict,
}

impl Failure {
    pub(super) fn message(self) -> &'static str {
        match self {
            Self::Unavailable => "Procurement storage unavailable",
            Self::Conflict => "shortage reference already used with different details",
        }
    }
}

#[derive(Deserialize)]
struct Record {
    proposal: Proposal,
}

pub(super) struct Store {
    drafts: Collection<Record>,
    scope: Scope,
}

impl Store {
    pub(super) fn new(database: &Database, scope: Scope) -> Self {
        Self {
            drafts: database.collection("drafts"),
            scope,
        }
    }

    fn key(&self, shortage_id: Uuid) -> Document {
        doc! { "_id": {
            "tenantId": self.scope.tenant_id.to_string(),
            "organizationId": self.scope.organization_id.to_string(),
            "shortageId": shortage_id.to_string(),
        }}
    }

    pub(super) async fn get(&self, shortage_id: Uuid) -> Result<Option<Proposal>, Failure> {
        self.drafts
            .find_one(self.key(shortage_id))
            .await
            .map(|record| record.map(|record| record.proposal))
            .map_err(|_| Failure::Unavailable)
    }

    pub(super) async fn record(&self, source: Shortage) -> Result<Proposal, Failure> {
        let key = self.key(source.shortage_id);
        let proposal = Proposal {
            proposal_id: Uuid::new_v4(),
            status: Status::Draft,
            source,
        };
        let payload = bson::to_document(&proposal).map_err(|_| Failure::Unavailable)?;
        // The scoped shortage reference is the unique inbox key and draft identity.
        // An insert and its deduplication marker are the same atomic write.
        let stored = self
            .drafts
            .find_one_and_update(
                key,
                doc! { "$setOnInsert": {
                    "proposal": payload,
                    "schemaVersion": 1,
                    "createdAt": DateTime::now(),
                    "actor": "procurement.local-fixture",
                }},
            )
            .upsert(true)
            .return_document(ReturnDocument::After)
            .await
            .map_err(|_| Failure::Unavailable)?
            .ok_or(Failure::Unavailable)?
            .proposal;
        if stored.source != proposal.source {
            return Err(Failure::Conflict);
        }
        Ok(stored)
    }
}
