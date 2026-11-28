use crate::{
    Scope,
    terms::{Command, Decision, Failure, Saved, Terms},
};
use mongodb::{
    Collection, Database, IndexModel,
    bson::{Document, doc},
    options::{CollectionOptions, IndexOptions, ReadConcern, WriteConcern},
};
use serde::Serialize;
use std::io;
use uuid::Uuid;

const REVISIONS: &str = "purchase_terms_revisions";

#[derive(Clone)]
pub(super) struct Store(Collection<Saved>);

fn scope_key(scope: Scope, plan_id: Uuid) -> Document {
    doc! {
        "tenantId": scope.tenant_id.to_string(),
        "organizationId": scope.organization_id.to_string(),
        "terms.plan_id": plan_id.to_string(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Record<'a> {
    #[serde(with = "uuid::serde::hyphenated")]
    tenant_id: Uuid,
    #[serde(with = "uuid::serde::hyphenated")]
    organization_id: Uuid,
    #[serde(flatten)]
    value: &'a Saved,
}

impl Store {
    pub(super) async fn open(db: &Database) -> io::Result<Self> {
        let options = CollectionOptions::builder()
            .read_concern(ReadConcern::majority())
            .write_concern(WriteConcern::majority())
            .build();
        let collection = db.collection_with_options(REVISIONS, options);
        let unique = |keys| {
            IndexModel::builder()
                .keys(keys)
                .options(IndexOptions::builder().unique(true).build())
                .build()
        };
        collection
            .create_indexes([
                unique(doc! {"tenantId": 1, "organizationId": 1, "terms.plan_id": 1, "terms.revision": -1}),
                unique(doc! {"tenantId": 1, "organizationId": 1, "terms.plan_id": 1, "commandId": 1}),
            ])
            .await
            .map_err(|_| io::Error::other("Cannot initialize Procurement terms indexes"))?;
        Ok(Self(collection))
    }

    pub(super) async fn latest(
        &self,
        scope: Scope,
        plan_id: Uuid,
    ) -> Result<Option<Terms>, Failure> {
        self.0
            .find_one(scope_key(scope, plan_id))
            .sort(doc! {"terms.revision": -1})
            .await
            .map(|stored| stored.map(|saved| saved.terms))
            .map_err(|_| Failure::Unavailable)
    }

    pub(super) async fn decide(
        &self,
        scope: Scope,
        plan_id: Uuid,
        command: &Command,
    ) -> Result<Decision, Failure> {
        // Read in this order so a same-command race reaching the latest query
        // also gets a chance to be recognized by the command lookup.
        let latest = self.latest(scope, plan_id).await?;
        let mut key = scope_key(scope, plan_id);
        key.insert("commandId", command.command_id.to_string());
        let saved = self
            .0
            .find_one(key)
            .await
            .map_err(|_| Failure::Unavailable)?;
        command.decide(latest.map_or(0, |terms| terms.revision), saved)
    }

    pub(super) async fn append(&self, scope: Scope, saved: &Saved) -> Result<(), Failure> {
        let record = Record {
            tenant_id: scope.tenant_id,
            organization_id: scope.organization_id,
            value: saved,
        };
        let document = mongodb::bson::to_document(&record).map_err(|_| Failure::Unavailable)?;
        // The immutable response doubles as the command receipt. Only one
        // append can claim expected_revision + 1 and only one can claim the
        // command ID in this scope. No mutable head or distributed transaction.
        self.0
            .clone_with_type::<Document>()
            .insert_one(document)
            .await
            .map_err(|_| Failure::Unavailable)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terms::tests::{command, fixture};
    use mongodb::bson;
    use std::collections::HashMap;

    #[test]
    fn bson_keeps_scoped_index_fields_audit_and_original_response() {
        let (scope, draft, body, supplier) = fixture();
        let command = command(&body).unwrap();
        let actor = Uuid::new_v4();
        let saved = command
            .bind(&draft)
            .unwrap()
            .snapshot(1, &HashMap::from([(supplier.id, supplier)]), actor)
            .unwrap();
        let doc = bson::to_document(&Record {
            tenant_id: scope.tenant_id,
            organization_id: scope.organization_id,
            value: &saved,
        })
        .unwrap();
        let plan_id = draft.plan.plan_id;
        assert_eq!(
            scope_key(scope, plan_id),
            doc! {
                "tenantId": scope.tenant_id.to_string(),
                "organizationId": scope.organization_id.to_string(),
                "terms.plan_id": plan_id.to_string(),
            }
        );
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
            assert_ne!(scope_key(scope, plan_id), scope_key(foreign, plan_id));
        }
        assert_ne!(scope_key(scope, plan_id), scope_key(scope, Uuid::new_v4()));
        assert_eq!(
            doc.get_str("tenantId").unwrap(),
            scope.tenant_id.to_string()
        );
        assert_eq!(
            doc.get_str("organizationId").unwrap(),
            scope.organization_id.to_string()
        );
        assert_eq!(
            doc.get_str("commandId").unwrap(),
            command.command_id.to_string()
        );
        assert_eq!(doc.get_str("actorId").unwrap(), actor.to_string());
        assert_eq!(doc.get_i32("expectedRevision"), Ok(0));
        assert_eq!(doc.get_i32("schemaVersion"), Ok(1));
        assert!(doc.get_datetime("savedAt").is_ok());
        let terms = doc.get_document("terms").unwrap();
        assert_eq!(terms.get_str("plan_id").unwrap(), plan_id.to_string());
        assert_eq!(terms.get_str("status"), Ok("draft"));
        assert_eq!(terms.get_i32("revision"), Ok(1));
        assert_eq!(terms.get_str("total"), Ok("162.42"));
        let line = terms.get_array("lines").unwrap()[0].as_document().unwrap();
        assert_eq!(line.get_str("pieces"), Ok("6"));
        assert_eq!(line.get_str("unit_price"), Ok("10.01"));
        assert_eq!(line.get_str("line_total"), Ok("60.06"));
        assert_eq!(line.get_document("supplier").unwrap().len(), 3);
        assert_eq!(bson::from_document::<Saved>(doc).unwrap(), saved);
    }
}
