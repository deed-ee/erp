use crate::domain::{Article, Scope};
use crate::units::Unit;
use mongodb::{
    Client, Collection, Database, IndexModel,
    bson::{Document, doc},
    error::{Error, ErrorKind, WriteFailure},
    options::IndexOptions,
};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct Store {
    database: Database,
    articles: Collection<Article>,
    outbox: Collection<Document>,
}

pub(crate) enum InsertError {
    Conflict,
    Database(Error),
}

impl Store {
    pub(crate) async fn open(uri: &str) -> mongodb::error::Result<Self> {
        let database = Client::with_uri_str(uri).await?.database("master_data");
        let articles = database.collection("articles");
        let outbox = database.collection("outbox");
        database
            .collection::<Unit>("units")
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "tenant_id": 1, "organization_id": 1, "code": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await?;
        outbox
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "messageId": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await?;
        articles
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "tenant_id": 1, "organization_id": 1, "sku": 1 })
                    .options(
                        IndexOptions::builder()
                            .name("article_scope_sku_v1".to_owned())
                            .unique(true)
                            .build(),
                    )
                    .build(),
            )
            .await?;
        Ok(Self {
            database,
            articles,
            outbox,
        })
    }

    pub(crate) async fn ping(&self) -> mongodb::error::Result<()> {
        self.database.run_command(doc! { "ping": 1 }).await?;
        Ok(())
    }

    pub(crate) async fn insert(
        &self,
        article: &Article,
        traceparent: &str,
    ) -> Result<(), InsertError> {
        let event = Self::article_registered(article, traceparent)?;
        let mut session = self
            .database
            .client()
            .start_session()
            .await
            .map_err(InsertError::Database)?;
        session
            .start_transaction()
            .await
            .map_err(InsertError::Database)?;
        if let Err(error) = self
            .articles
            .insert_one(article)
            .session(&mut session)
            .await
        {
            let _ = session.abort_transaction().await;
            return Err(Self::classify_insert(error));
        }
        if let Err(error) = self.outbox.insert_one(event).session(&mut session).await {
            let _ = session.abort_transaction().await;
            return Err(InsertError::Database(error));
        }
        session
            .commit_transaction()
            .await
            .map_err(InsertError::Database)
    }

    fn article_registered(article: &Article, traceparent: &str) -> Result<Document, InsertError> {
        let timestamp = article
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|error| InsertError::Database(Error::custom(error)))?;
        let message_id = Uuid::new_v4().to_string();
        Ok(doc! {
            "messageId": &message_id,
            "messageType": "business.master-data.article.registered.v1",
            "schemaVersion": 1,
            "timestamp": timestamp,
            "tenantId": article.tenant_id.to_string(),
            "organizationId": article.organization_id.to_string(),
            "actor": { "kind": "user", "id": article.created_by.to_string() },
            "correlationId": message_id,
            "traceparent": traceparent,
            "sourceService": "erp4us.master-data",
            "payload": { "articleId": article.id.to_string(), "revision": article.revision },
        })
    }

    pub(crate) async fn insert_unit(&self, unit: &Unit) -> Result<(), InsertError> {
        self.database
            .collection::<Unit>("units")
            .insert_one(unit)
            .await
            .map(|_| ())
            .map_err(Self::classify_insert)
    }

    pub(crate) async fn get_unit(
        &self,
        scope: Scope,
        id: Uuid,
    ) -> mongodb::error::Result<Option<Unit>> {
        self.database
            .collection::<Unit>("units")
            .find_one(Self::scoped_id(scope, id))
            .await
    }

    pub(crate) async fn get(
        &self,
        scope: Scope,
        id: Uuid,
    ) -> mongodb::error::Result<Option<Article>> {
        self.articles.find_one(Self::scoped_id(scope, id)).await
    }

    fn scoped_id(scope: Scope, id: Uuid) -> Document {
        doc! {
            "_id": id.to_string(),
            "tenant_id": scope.tenant.to_string(),
            "organization_id": scope.organization.to_string(),
        }
    }

    fn classify_insert(error: Error) -> InsertError {
        match error.kind.as_ref() {
            ErrorKind::Write(WriteFailure::WriteError(e)) if e.code == 11000 => {
                InsertError::Conflict
            }
            _ => InsertError::Database(error),
        }
    }
}
