use crate::domain::{CreateDraft, Draft};
use erp4us_identity::Scope;
use mongodb::{
    Collection, Database, IndexModel,
    bson::{Document, doc},
    error::{Error, ErrorKind, WriteFailure},
    options::IndexOptions,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub(crate) struct SavedOrder {
    schema_version: u32,
    tenant_id: String,
    organization_id: String,
    created_by: String,
    input: CreateDraft,
    pub draft: Draft,
}

impl SavedOrder {
    pub(crate) fn new(scope: Scope, input: CreateDraft, draft: Draft) -> Self {
        Self {
            schema_version: 1,
            tenant_id: scope.tenant.to_string(),
            organization_id: scope.organization.to_string(),
            created_by: scope.actor.to_string(),
            input,
            draft,
        }
    }

    pub(crate) fn is_retry(&self, scope: Scope, input: &CreateDraft) -> bool {
        self.created_by == scope.actor.to_string() && &self.input == input
    }

    pub(crate) fn is_same_command_as(&self, other: &Self) -> bool {
        self.created_by == other.created_by && self.input == other.input
    }
}

#[derive(Clone)]
pub(crate) struct Store {
    db: Database,
    orders: Collection<SavedOrder>,
}

pub(crate) enum Insert {
    Created,
    AlreadyExists,
}

fn scoped(scope: Scope) -> Document {
    doc! { "tenant_id": scope.tenant.to_string(), "organization_id": scope.organization.to_string() }
}

impl Store {
    pub(crate) async fn open(db: Database) -> Result<Self, Error> {
        let orders = db.collection("orders");
        orders
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "tenant_id": 1, "organization_id": 1, "draft.id": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await?;
        Ok(Self { db, orders })
    }

    pub(crate) async fn ready(&self) -> bool {
        self.db.run_command(doc! {"ping": 1}).await.is_ok()
    }

    pub(crate) async fn insert(&self, order: &SavedOrder) -> Result<Insert, Error> {
        match self.orders.insert_one(order).await {
            Ok(_) => Ok(Insert::Created),
            Err(error) => match error.kind.as_ref() {
                ErrorKind::Write(WriteFailure::WriteError(error)) if error.code == 11000 => {
                    Ok(Insert::AlreadyExists)
                }
                _ => Err(error),
            },
        }
    }

    pub(crate) async fn get(&self, scope: Scope, id: Uuid) -> Result<Option<SavedOrder>, Error> {
        let mut filter = scoped(scope);
        filter.insert("draft.id", id.to_string());
        self.orders.find_one(filter).await
    }

    pub(crate) async fn page(
        &self,
        scope: Scope,
        after: Option<Uuid>,
        limit: usize,
    ) -> Result<(Vec<Draft>, Option<Uuid>), Error> {
        let mut filter = scoped(scope);
        if let Some(after) = after {
            filter.insert("draft.id", doc! { "$gt": after.to_string() });
        }
        let mut items: Vec<Draft> = Vec::with_capacity(limit);
        let mut cursor = self
            .orders
            .find(filter)
            .sort(doc! {"draft.id": 1})
            .limit(limit as i64 + 1)
            .await?;
        while cursor.advance().await? {
            if items.len() == limit {
                let next = items.last().map(|draft| draft.id);
                return Ok((items, next));
            }
            items.push(cursor.deserialize_current()?.draft);
        }
        Ok((items, None))
    }
}
