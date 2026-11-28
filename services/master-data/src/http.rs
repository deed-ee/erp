use crate::{
    auth::{Auth, Denial, Permission},
    domain::{Article, ArticleNameMatch, RegisterArticle, Scope, valid_article_name, valid_sku},
    partners,
    store::{InsertError, Store},
};
use axum::{
    Extension, Json, Router,
    extract::{
        DefaultBodyLimit, Path, Query, Request, State,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub(crate) fn router(store: Store, auth: Auth) -> Router {
    Router::new()
        .route(
            "/v1/articles:register",
            post(register).layer(middleware::from_fn_with_state(
                (auth.clone(), Permission::Register),
                authorize,
            )),
        )
        .route(
            "/v1/articles:find-by-sku",
            get(find_by_sku)
                .head(|| async { StatusCode::NO_CONTENT })
                .layer(middleware::from_fn_with_state(
                    (auth.clone(), Permission::Read),
                    authorize,
                )),
        )
        .route(
            "/v1/articles:find-by-name",
            get(find_by_name)
                .head(|| async { StatusCode::NO_CONTENT })
                .layer(middleware::from_fn_with_state(
                    (auth.clone(), Permission::Read),
                    authorize,
                )),
        )
        .route(
            "/v1/articles/{id}",
            get(find).layer(middleware::from_fn_with_state(
                (auth.clone(), Permission::Read),
                authorize,
            )),
        )
        .route(
            "/v1/units:register",
            post(crate::units::register).layer(middleware::from_fn_with_state(
                (auth.clone(), Permission::RegisterUnit),
                authorize,
            )),
        )
        .route(
            "/v1/units/{id}",
            get(crate::units::find).layer(middleware::from_fn_with_state(
                (auth.clone(), Permission::ReadUnit),
                authorize,
            )),
        )
        .route(
            "/v1/partners:register",
            post(partners::register).layer(middleware::from_fn_with_state(
                (auth.clone(), Permission::RegisterPartner),
                authorize,
            )),
        )
        .route(
            "/v1/partners/{id}",
            get(partners::find).layer(middleware::from_fn_with_state(
                (auth, Permission::ReadPartner),
                authorize,
            )),
        )
        .layer(DefaultBodyLimit::max(4096))
        .route("/health/live", get(|| async { StatusCode::NO_CONTENT }))
        .route("/health/ready", get(ready))
        .with_state(store)
}

async fn authorize(
    State((auth, permission)): State<(Auth, Permission)>,
    mut request: Request,
    next: Next,
) -> Response {
    let mut response = match auth.authorize(request.headers(), permission) {
        Ok(scope) => {
            request.extensions_mut().insert(scope);
            next.run(request).await
        }
        Err(Denial::Unauthenticated) => (
            [(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"))],
            problem(
                StatusCode::UNAUTHORIZED,
                "A valid bearer access token is required",
            ),
        )
            .into_response(),
        Err(Denial::InvalidScope) => problem(
            StatusCode::BAD_REQUEST,
            "Valid tenant and organization headers are required",
        ),
        Err(Denial::Forbidden) => problem(
            StatusCode::FORBIDDEN,
            "Operation not permitted in the requested scope",
        ),
    };
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
    Extension(scope): Extension<Scope>,
    body: Result<Json<RegisterArticle>, JsonRejection>,
) -> Response {
    let Json(command) = match body {
        Ok(body) => body,
        Err(rejection) => return problem(rejection.status(), "Invalid JSON request"),
    };
    let article = match Article::register(scope, command) {
        Ok(article) => article,
        Err(detail) => return problem(StatusCode::UNPROCESSABLE_ENTITY, detail),
    };
    let span_id = Uuid::new_v4().simple().to_string();
    let traceparent = format!("00-{}-{}-00", Uuid::new_v4().simple(), &span_id[..16]);
    match store.insert(&article, &traceparent).await {
        Ok(()) => {
            tracing::info!(%traceparent, "Article registration committed");
            (
                StatusCode::CREATED,
                [(header::LOCATION, format!("/v1/articles/{}", article.id))],
                [("traceparent", traceparent)],
                Json(ArticleView::from(article)),
            )
                .into_response()
        }
        Err(InsertError::Conflict) => problem(StatusCode::CONFLICT, "SKU already registered"),
        Err(InsertError::Database(error)) => unavailable("register_article", error),
    }
}

async fn find(
    State(store): State<Store>,
    Extension(scope): Extension<Scope>,
    Path(id): Path<String>,
) -> Response {
    let Ok(id) = Uuid::parse_str(&id) else {
        return problem(StatusCode::BAD_REQUEST, "Invalid article ID");
    };
    article_response(store.get(scope, id).await, "find_article")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkuQuery {
    sku: String,
}

async fn find_by_sku(
    State(store): State<Store>,
    Extension(scope): Extension<Scope>,
    query: Result<Query<SkuQuery>, QueryRejection>,
) -> Response {
    let Ok(Query(SkuQuery { sku })) = query else {
        return problem(StatusCode::BAD_REQUEST, "Provide exactly one SKU parameter");
    };
    if !valid_sku(&sku) {
        return problem(StatusCode::BAD_REQUEST, "Invalid SKU");
    }
    article_response(store.find_by_sku(scope, &sku).await, "find_article_by_sku")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NameQuery {
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NameMatches {
    matches: Vec<ArticleNameMatch>,
    has_more: bool,
}

async fn find_by_name(
    State(store): State<Store>,
    Extension(scope): Extension<Scope>,
    query: Result<Query<NameQuery>, QueryRejection>,
) -> Response {
    let Ok(Query(NameQuery { name })) = query else {
        return problem(
            StatusCode::BAD_REQUEST,
            "Provide exactly one Article name parameter",
        );
    };
    if !valid_article_name(&name) {
        return problem(StatusCode::BAD_REQUEST, "Invalid Article name");
    }
    match store.find_by_name(scope, &name).await {
        Ok((matches, has_more)) => Json(NameMatches { matches, has_more }).into_response(),
        Err(error) => unavailable("find_articles_by_name", error),
    }
}

fn article_response(result: mongodb::error::Result<Option<Article>>, operation: &str) -> Response {
    match result {
        Ok(Some(article)) => Json(ArticleView::from(article)).into_response(),
        Ok(None) => problem(StatusCode::NOT_FOUND, "Article not found"),
        Err(error) => unavailable(operation, error),
    }
}

pub(crate) fn unavailable(operation: &str, _error: mongodb::error::Error) -> Response {
    tracing::error!(operation, "MongoDB operation failed");
    problem(StatusCode::SERVICE_UNAVAILABLE, "Persistence unavailable")
}

#[derive(Serialize)]
struct Problem {
    status: u16,
    detail: &'static str,
}

pub(crate) fn problem(status: StatusCode, detail: &'static str) -> Response {
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
