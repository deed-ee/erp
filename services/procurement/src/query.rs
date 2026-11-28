use crate::{
    Scope,
    drafts::{Draft, Status, Store},
    material_plan::{Bom, BuyRow},
    required,
};
use axum::{
    Json, Router,
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    routing::get,
};
use erp4us_identity::{Auth, Denial};
use serde::Serialize;
use std::{env, io, time::Duration};
use tokio::time::timeout;
use uuid::Uuid;

const LOOKUP_TIMEOUT: Duration = Duration::from_secs(5);
const READ_PERMISSION: &str = "procurement.proposal.read";

#[derive(Clone)]
struct Api {
    store: Store,
    auth: Auth,
}

#[derive(Serialize)]
struct Proposal {
    proposal_id: Uuid,
    plan_id: Uuid,
    order_id: Uuid,
    line_id: Uuid,
    status: Status,
    bom: Bom,
    lines: Vec<BuyRow>,
}

impl From<Draft> for Proposal {
    fn from(draft: Draft) -> Self {
        Self {
            proposal_id: draft.proposal_id,
            plan_id: draft.plan.plan_id,
            order_id: draft.plan.order_id,
            line_id: draft.plan.line_id,
            status: draft.status,
            bom: draft.plan.bom,
            lines: draft.plan.buy_rows,
        }
    }
}

fn authorize(auth: &Auth, headers: &HeaderMap) -> Result<Scope, StatusCode> {
    let scope = auth
        .authorize(headers, READ_PERMISSION)
        .map_err(|error| match error {
            Denial::Unauthenticated => StatusCode::UNAUTHORIZED,
            Denial::InvalidScope => StatusCode::BAD_REQUEST,
            Denial::Forbidden => StatusCode::FORBIDDEN,
        })?;
    if scope.tenant.is_nil() || scope.organization.is_nil() {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(Scope {
        tenant_id: scope.tenant,
        organization_id: scope.organization,
    })
}

async fn lookup(
    State(api): State<Api>,
    headers: HeaderMap,
    Path(plan_id): Path<String>,
) -> Result<Json<Proposal>, StatusCode> {
    let scope = authorize(&api.auth, &headers)?;
    let plan_id = Uuid::try_parse(&plan_id)
        .ok()
        .filter(|id| !id.is_nil())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let draft = timeout(LOOKUP_TIMEOUT, api.store.get(scope, plan_id))
        .await
        .ok()
        .and_then(Result::ok)
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(draft.into()))
}

pub(super) async fn serve() -> io::Result<()> {
    let auth = Auth::load(
        &required("ERP4US_IDENTITY_POLICY")?,
        &required("ERP4US_ISSUER_JWKS")?,
    )?;
    let store = Store::open().await?;
    let ready = store.clone();
    let bind = env::var("ERP4US_BIND").unwrap_or_else(|_| {
        let port = env::var("ERP4US_PROCUREMENT_PORT").unwrap_or_else(|_| "8094".into());
        format!("127.0.0.1:{port}")
    });
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(
        listener,
        Router::new()
            .route("/health/live", get(|| async { StatusCode::NO_CONTENT }))
            .route(
                "/health/ready",
                get(move || async move {
                    if timeout(LOOKUP_TIMEOUT, ready.ready()).await == Ok(true) {
                        StatusCode::NO_CONTENT
                    } else {
                        StatusCode::SERVICE_UNAVAILABLE
                    }
                }),
            )
            .route("/v1/procurement/proposals/by-plan/{plan_id}", get(lookup))
            .layer(middleware::from_fn(
                |request: Request, next: Next| async move {
                    let mut response = next.run(request).await;
                    let status = response.status();
                    let headers = response.headers_mut();
                    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                    if status == StatusCode::UNAUTHORIZED {
                        headers
                            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
                    }
                    response
                },
            ))
            .with_state(Api { store, auth }),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material_plan::tests::{read, sample};
    use mongodb::bson::{DateTime, doc, from_document, to_document};
    use serde_json::json;

    #[test]
    fn public_response_has_only_draft_snapshot_fields() {
        let sample = sample();
        let event = read(&sample).unwrap();
        let proposal_id = Uuid::new_v4();
        let draft: Draft = from_document(doc! {
            "proposalId": proposal_id.to_string(),
            "status": "draft",
            "plan": to_document(&event.plan).unwrap(),
            "sourceEventId": event.message_id.to_string(),
            "origin": to_document(&event.origin).unwrap(),
            "createdAt": DateTime::now(),
            "schemaVersion": 1,
        })
        .unwrap();
        let response = serde_json::to_value(Proposal::from(draft)).unwrap();
        assert_eq!(
            response,
            json!({
                "proposal_id": proposal_id,
                "plan_id": sample["payload"]["planId"],
                "order_id": sample["payload"]["orderId"],
                "line_id": sample["payload"]["lineId"],
                "status": "draft",
                "bom": {"sku":"NM-1000","revision":"A"},
                "lines": [
                    {"sku":"NM-MOTOR","revision":null,"unit":"EA","pieces":"6"},
                    {"sku":"NM-SPINDLE","revision":null,"unit":"EA","pieces":"10"},
                    {"sku":"NM-SPINDLE","revision":"B","unit":"EA","pieces":"2"}
                ]
            })
        );
    }
}
