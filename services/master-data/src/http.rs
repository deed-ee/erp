use crate::{
    domain::{Article, RegisterArticle, Scope},
    store::{InsertError, Store},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Request, State, rejection::JsonRejection},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use subtle::ConstantTimeEq;
use time::OffsetDateTime;
use uuid::{Uuid, uuid};

const DEMO_SCOPE: Scope = Scope {
    tenant: uuid!("ddcae139-c1e8-47d2-8b48-349cb6f30ce5"),
    organization: uuid!("3e793fb5-75a7-45f6-8f66-45a58fcec674"),
    actor: uuid!("242a290f-4bbc-438e-aefe-e3697a8bcd16"),
};

pub(crate) fn router(store: Store, token: String) -> Router {
    Router::new()
        .route("/v1/articles:register", post(register))
        .route("/v1/articles/{id}", get(find))
        .route_layer(middleware::from_fn_with_state(token, authenticate))
        .layer(DefaultBodyLimit::max(4096))
        .route("/health/live", get(|| async { StatusCode::NO_CONTENT }))
        .route("/health/ready", get(ready))
        .with_state(store)
}

async fn authenticate(State(token): State<String>, request: Request, next: Next) -> Response {
    let accepted = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .is_some_and(|(scheme, candidate)| {
            scheme.eq_ignore_ascii_case("bearer")
                && bool::from(token.as_bytes().ct_eq(candidate.as_bytes()))
        });
    if !accepted {
        return (
            [(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"))],
            problem(
                StatusCode::UNAUTHORIZED,
                "A valid demo bearer token is required",
            ),
        )
            .into_response();
    }
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn ready(State(store): State<Store>) -> StatusCode {
    match store.ping().await {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

#[derive(Serialize)]
struct ArticleView {
    id: Uuid,
    tenant_id: Uuid,
    organization_id: Uuid,
    created_by: Uuid,
    schema_version: u32,
    revision: u32,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    sku: String,
    name: String,
}

impl From<Article> for ArticleView {
    fn from(article: Article) -> Self {
        Self {
            id: article.id,
            tenant_id: article.tenant_id,
            organization_id: article.organization_id,
            created_by: article.created_by,
            schema_version: article.schema_version,
            revision: article.revision,
            created_at: article.created_at,
            sku: article.sku,
            name: article.name,
        }
    }
}

async fn register(
    State(store): State<Store>,
    body: Result<Json<RegisterArticle>, JsonRejection>,
) -> Response {
    let Json(command) = match body {
        Ok(body) => body,
        Err(rejection) => return problem(rejection.status(), "Invalid JSON request"),
    };
    let article = match Article::register(DEMO_SCOPE, command) {
        Ok(article) => article,
        Err(detail) => return problem(StatusCode::UNPROCESSABLE_ENTITY, detail),
    };
    match store.insert(&article).await {
        Ok(()) => (
            StatusCode::CREATED,
            [(header::LOCATION, format!("/v1/articles/{}", article.id))],
            Json(ArticleView::from(article)),
        )
            .into_response(),
        Err(InsertError::Conflict) => problem(StatusCode::CONFLICT, "SKU already registered"),
        Err(InsertError::Database(error)) => unavailable("register_article", error),
    }
}

async fn find(State(store): State<Store>, Path(id): Path<String>) -> Response {
    let Ok(id) = Uuid::parse_str(&id) else {
        return problem(StatusCode::BAD_REQUEST, "Invalid article ID");
    };
    match store.get(DEMO_SCOPE, id).await {
        Ok(Some(article)) => Json(ArticleView::from(article)).into_response(),
        Ok(None) => problem(StatusCode::NOT_FOUND, "Article not found"),
        Err(error) => unavailable("find_article", error),
    }
}

fn unavailable(operation: &str, _error: mongodb::error::Error) -> Response {
    tracing::error!(operation, "MongoDB operation failed");
    problem(StatusCode::SERVICE_UNAVAILABLE, "Persistence unavailable")
}

#[derive(Serialize)]
struct Problem {
    status: u16,
    detail: &'static str,
}

fn problem(status: StatusCode, detail: &'static str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/problem+json")],
        Json(Problem {
            status: status.as_u16(),
            detail,
        }),
    )
        .into_response()
}
