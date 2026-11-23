use crate::domain::{Article, Scope};
use mongodb::{
    Client, Collection, Database, IndexModel,
    bson::doc,
    error::{Error, ErrorKind, WriteFailure},
    options::IndexOptions,
};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct Store {
    database: Database,
    articles: Collection<Article>,
}

pub(crate) enum InsertError {
    Conflict,
    Database(Error),
}

impl Store {
    pub(crate) async fn open(uri: &str) -> mongodb::error::Result<Self> {
        let database = Client::with_uri_str(uri).await?.database("master_data");
        let articles = database.collection("articles");
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
        Ok(Self { database, articles })
    }

    pub(crate) async fn ping(&self) -> mongodb::error::Result<()> {
        self.database.run_command(doc! { "ping": 1 }).await?;
        Ok(())
    }

    pub(crate) async fn insert(&self, article: &Article) -> Result<(), InsertError> {
        self.articles
            .insert_one(article)
            .await
            .map(|_| ())
            .map_err(|error| match error.kind.as_ref() {
                ErrorKind::Write(WriteFailure::WriteError(e)) if e.code == 11000 => {
                    InsertError::Conflict
                }
                _ => InsertError::Database(error),
            })
    }

    pub(crate) async fn get(
        &self,
        scope: Scope,
        id: Uuid,
    ) -> mongodb::error::Result<Option<Article>> {
        self.articles
            .find_one(doc! {
                "_id": id.to_string(),
                "tenant_id": scope.tenant.to_string(),
                "organization_id": scope.organization.to_string(),
            })
            .await
    }
}
