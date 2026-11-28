use crate::domain::{ARTICLE_NAME_MATCH_LIMIT, Article, ArticleNameMatch, Scope};
use crate::units::Unit;
use mongodb::{
    Client, Collection, Database, IndexModel,
    bson::{Document, doc},
    error::{Error, ErrorKind, WriteFailure},
    options::{Collation, Hint, IndexOptions},
};
use std::time::Duration;
use uuid::Uuid;

const ARTICLE_NAME_INDEX: &str = "article_scope_name_sku_v1";

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
    pub(crate) fn database(&self) -> &Database {
        &self.database
    }

    pub(crate) async fn open(uri: &str) -> mongodb::error::Result<Self> {
        let database = Client::with_uri_str(uri).await?.database("master_data");
        crate::partners::ensure_index(&database).await?;
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
        articles
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "tenant_id": 1, "organization_id": 1, "name": 1, "sku": 1 })
                    .options(
                        IndexOptions::builder()
                            .name(ARTICLE_NAME_INDEX.to_owned())
                            .collation(Self::binary_collation())
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
        use erp4us_messaging::{Kind, Layer, Route, Scope};
        let subject = Route {
            layer: Layer::Business,
            domain: "master-data",
            kind: Kind::Event,
            resource: "article",
            action: "registered",
        }
        .subject(1, Scope::Tenant(article.tenant_id), 1)
        .map_err(|error| InsertError::Database(Error::custom(error)))?;
        Ok(doc! {
            "messageId": &message_id,
            "messageType": subject,
            "protocolVersion": "1.0.0",
            "schemaVersion": "1.0.0",
            "serviceVersion": env!("CARGO_PKG_VERSION"),
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

    pub(crate) async fn find_by_sku(
        &self,
        scope: Scope,
        sku: &str,
    ) -> mongodb::error::Result<Option<Article>> {
        let mut selector = Self::scope_selector(scope);
        selector.insert("sku", sku);
        self.articles.find_one(selector).await
    }

    pub(crate) async fn find_by_name(
        &self,
        scope: Scope,
        name: &str,
    ) -> mongodb::error::Result<(Vec<ArticleNameMatch>, bool)> {
        let mut selector = Self::scope_selector(scope);
        selector.insert("name", name);
        let mut cursor = self
            .articles
            .clone_with_type::<ArticleNameMatch>()
            .find(selector)
            .projection(doc! { "_id": 1, "sku": 1, "name": 1, "revision": 1 })
            .sort(doc! { "sku": 1 })
            .collation(Self::binary_collation())
            .hint(Hint::Name(ARTICLE_NAME_INDEX.to_owned()))
            .max_time(Duration::from_secs(2))
            .limit((ARTICLE_NAME_MATCH_LIMIT + 1) as i64)
            .await?;
        let mut matches = Vec::with_capacity(ARTICLE_NAME_MATCH_LIMIT);
        while matches.len() < ARTICLE_NAME_MATCH_LIMIT && cursor.advance().await? {
            matches.push(cursor.deserialize_current()?);
        }
        let has_more = matches.len() == ARTICLE_NAME_MATCH_LIMIT && cursor.advance().await?;
        Ok((matches, has_more))
    }

    fn binary_collation() -> Collation {
        Collation::builder().locale("simple").build()
    }

    fn scoped_id(scope: Scope, id: Uuid) -> Document {
        let mut selector = Self::scope_selector(scope);
        selector.insert("_id", id.to_string());
        selector
    }

    fn scope_selector(scope: Scope) -> Document {
        doc! {
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
