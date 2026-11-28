//! HTTP routes for tests and the explicit synthetic receiving proof.

use super::{
    Binding, Organization, PurchaseLine, Reject, SalesLine,
    application::{Failure, Line, Owner, Receiving},
    mongo::{self, Allocation, Intent, Post, Prepare, Saved, Store},
    source::verified_scope,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRequest, Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use erp4us_identity::Auth;
use serde::{Deserialize, Deserializer, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

const RECEIPTS: &str = "/v1/inventory/reservations/{order_id}/{line_id}/material-plan/purchase-orders/{purchase_order_id}/lines/{purchase_line_id}/receipts";
const INTENT: &str = "/v1/inventory/reservations/{order_id}/{line_id}/material-plan/purchase-orders/{purchase_order_id}/lines/{purchase_line_id}/receipts/intent";
const READ_PERMISSION: &str = "inventory.purchase-receipt.read";
const UNAVAILABLE: StatusCode = StatusCode::SERVICE_UNAVAILABLE;

#[derive(Clone)]
struct Api {
    store: Store,
    auth: Auth,
    peer: Owner,
}

pub(super) fn router(store: Store, auth: Auth, peer: Owner) -> Router {
    Router::new()
        .route(RECEIPTS, get(allocation).post(confirm))
        .route(INTENT, get(reopen).post(prepare))
        .layer(DefaultBodyLimit::max(1024))
        .layer(middleware::from_fn(private_response))
        .with_state(Api { store, auth, peer })
}

async fn private_response(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if response.status() == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    response
}

#[derive(Deserialize)]
struct ReceiptPath {
    order_id: String,
    line_id: String,
    purchase_order_id: String,
    purchase_line_id: String,
}

impl ReceiptPath {
    fn parse(&self) -> Result<Line, Rejection> {
        Ok(Line {
            order: id(&self.order_id)?,
            line: id(&self.line_id)?,
            purchase: PurchaseLine {
                order: id(&self.purchase_order_id)?,
                line: id(&self.purchase_line_id)?,
            },
        })
    }
}

fn id(value: &str) -> Result<Uuid, Rejection> {
    Uuid::try_parse(value)
        .ok()
        .filter(|parsed| !parsed.is_nil() && parsed.to_string() == value)
        .ok_or(Rejection::Status(StatusCode::BAD_REQUEST))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareBody {
    pieces: String,
    #[serde(deserialize_with = "explicit_acknowledgment")]
    acknowledged_receipt_id: Option<String>,
}

fn explicit_acknowledgment<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::deserialize(deserializer)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmBody {
    receipt_id: String,
    pieces: String,
}

async fn body<T: serde::de::DeserializeOwned>(
    request: Request,
    state: &Api,
) -> Result<T, Rejection> {
    Json::<T>::from_request(request, state)
        .await
        .map(|Json(value)| value)
        .map_err(|failure| {
            if failure.status() == StatusCode::PAYLOAD_TOO_LARGE {
                StatusCode::PAYLOAD_TOO_LARGE
            } else {
                StatusCode::BAD_REQUEST
            }
            .into()
        })
}

async fn prepare(
    State(state): State<Api>,
    Path(path): Path<ReceiptPath>,
    request: Request,
) -> Result<(StatusCode, Json<IntentView>), Rejection> {
    let receiving = Receiving::authorize(&state.store, &state.auth, request.headers())?;
    let line = path.parse()?;
    let input: PrepareBody = body(request, &state).await?;
    let ack = input
        .acknowledged_receipt_id
        .as_deref()
        .map(id)
        .transpose()?;
    let (status, intent) = match receiving
        .prepare(line, ack, &input.pieces, state.peer)
        .await?
    {
        Prepare::Created(intent) => (StatusCode::CREATED, intent),
        Prepare::Replay(intent) => (StatusCode::OK, intent),
    };
    Ok((status, Json(IntentView::try_from(intent)?)))
}

async fn reopen(
    State(state): State<Api>,
    Path(path): Path<ReceiptPath>,
    headers: HeaderMap,
) -> Result<Json<IntentView>, Rejection> {
    let receiving = Receiving::authorize(&state.store, &state.auth, &headers)?;
    let intent = receiving
        .reopen(path.parse()?)
        .await?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(IntentView::try_from(intent)?))
}

async fn confirm(
    State(state): State<Api>,
    Path(path): Path<ReceiptPath>,
    request: Request,
) -> Result<(StatusCode, Json<ReceiptView>), Rejection> {
    let receiving = Receiving::authorize(&state.store, &state.auth, request.headers())?;
    let line = path.parse()?;
    let input: ConfirmBody = body(request, &state).await?;
    let (status, receipt) = match receiving
        .confirm(line, id(&input.receipt_id)?, &input.pieces, state.peer)
        .await?
    {
        Post::Created(receipt) => (StatusCode::CREATED, receipt),
        Post::Replay(receipt) => (StatusCode::OK, receipt),
    };
    Ok((status, Json(ReceiptView::try_from(receipt)?)))
}

async fn allocation(
    State(state): State<Api>,
    Path(path): Path<ReceiptPath>,
    headers: HeaderMap,
) -> Result<Json<AllocationView>, Rejection> {
    let scope = verified_scope(&state.auth, &headers, READ_PERMISSION)?;
    let line = path.parse()?;
    let sales = SalesLine {
        scope: Organization {
            tenant: scope.tenant,
            organization: scope.organization,
        },
        order: line.order,
        line: line.line,
    };
    let (plan, _) = state.store.plan_destination(sales).await?;
    let (binding, allocation) = state
        .store
        .allocation_snapshot(sales, plan, line.purchase)
        .await?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(AllocationView::from_snapshot(&binding, allocation)))
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
enum IntentView {
    Prepared { receipt_id: Uuid, pieces: String },
    Confirmed { receipt: Box<ReceiptView> },
}

impl TryFrom<Intent> for IntentView {
    type Error = Rejection;

    fn try_from(intent: Intent) -> Result<Self, Self::Error> {
        if let Some(receipt) = intent.confirmed {
            Ok(Self::Confirmed {
                receipt: Box::new(ReceiptView::try_from(*receipt)?),
            })
        } else {
            Ok(Self::Prepared {
                receipt_id: intent.command.receipt_id,
                pieces: intent.command.pieces.to_string(),
            })
        }
    }
}

#[derive(Serialize)]
struct ReceiptView {
    receipt_id: Uuid,
    pieces: String,
    posted_at: String,
    allocation_after: AllocationView,
}

impl TryFrom<Saved> for ReceiptView {
    type Error = Rejection;

    fn try_from(receipt: Saved) -> Result<Self, Self::Error> {
        let millis = i128::from(receipt.posted_at.timestamp_millis());
        let timestamp = OffsetDateTime::from_unix_timestamp_nanos(millis * 1_000_000)
            .map_err(|_| UNAVAILABLE)?;
        let allocation = Allocation::from_effect(&receipt.effect);
        Ok(Self {
            receipt_id: receipt.effect.command.receipt_id,
            pieces: receipt.effect.command.pieces.to_string(),
            posted_at: timestamp.format(&Rfc3339).map_err(|_| UNAVAILABLE)?,
            allocation_after: AllocationView::from_snapshot(
                &receipt.effect.line.binding,
                allocation,
            ),
        })
    }
}

#[derive(Serialize)]
struct AllocationView {
    plan_id: Uuid,
    purchase_order_id: Uuid,
    purchase_line_id: Uuid,
    sku: String,
    revision: Option<String>,
    unit: &'static str,
    through_receipt_sequence: i32,
    receipt_allocated_pieces: String,
    line_open_pieces: String,
    plan_held_pieces: String,
    plan_remaining_shortage_pieces: String,
}

impl AllocationView {
    fn from_snapshot(binding: &Binding, allocation: Allocation) -> Self {
        let row = &binding.row.key;
        Self {
            plan_id: row.plan,
            purchase_order_id: binding.purchase.order,
            purchase_line_id: binding.purchase.line,
            sku: row.stock.item.sku.clone(),
            revision: row.stock.item.revision.clone(),
            unit: "EA",
            through_receipt_sequence: allocation.sequence,
            receipt_allocated_pieces: allocation.allocated.to_string(),
            line_open_pieces: allocation.open.to_string(),
            plan_held_pieces: allocation.held.to_string(),
            plan_remaining_shortage_pieces: allocation.shortage.to_string(),
        }
    }
}

enum Rejection {
    Status(StatusCode),
    Conflict(&'static str),
}

impl From<StatusCode> for Rejection {
    fn from(status: StatusCode) -> Self {
        Self::Status(status)
    }
}

impl From<Failure> for Rejection {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Request(error) => Self::from(error),
            Failure::Storage(error) => Self::from(error),
            Failure::Owner(status) => Self::from(status),
        }
    }
}

impl From<mongo::Failure> for Rejection {
    fn from(failure: mongo::Failure) -> Self {
        match failure {
            mongo::Failure::Rejected(error) => Self::from(error),
            mongo::Failure::Unavailable => Self::from(UNAVAILABLE),
        }
    }
}

impl From<Reject> for Rejection {
    fn from(error: Reject) -> Self {
        match error {
            Reject::InvalidCommand => Self::from(StatusCode::BAD_REQUEST),
            Reject::NotFound | Reject::ActorMismatch => Self::from(StatusCode::NOT_FOUND),
            Reject::IntentNotCurrent => Self::Conflict("receipt_intent_not_current"),
            Reject::IntentChanged => Self::Conflict("receipt_intent_changed"),
            Reject::OverReceipt => Self::Conflict("over_receipt"),
            Reject::PlanIneligible => Self::Conflict("plan_ineligible"),
            Reject::InvalidSource | Reject::UnverifiedDestination | Reject::InvalidState => {
                Self::from(UNAVAILABLE)
            }
        }
    }
}

impl IntoResponse for Rejection {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorView {
            code: &'static str,
        }
        match self {
            Self::Status(status) => status.into_response(),
            Self::Conflict(code) => {
                (StatusCode::CONFLICT, Json(ErrorView { code })).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_only_the_explicit_intent_and_confirm_fields() {
        assert!(
            serde_json::from_value::<PrepareBody>(
                json!({"pieces": "6", "acknowledged_receipt_id": null})
            )
            .unwrap()
            .acknowledged_receipt_id
            .is_none()
        );
        for data in [
            json!({"pieces": "6"}),
            json!({"pieces": 6, "acknowledged_receipt_id": null}),
            json!({"pieces": "6", "acknowledged_receipt_id": null, "warehouse_id": "x"}),
        ] {
            assert!(serde_json::from_value::<PrepareBody>(data).is_err());
        }
        assert!(
            serde_json::from_value::<ConfirmBody>(
                json!({"receipt_id": Uuid::new_v4(), "pieces": "6", "actor_id": "x"})
            )
            .is_err()
        );
        let canonical = Uuid::from_u128(0xabc);
        assert!(id(&canonical.to_string()).is_ok());
        assert!(id(&canonical.to_string().to_uppercase()).is_err());
        assert!(id(&canonical.simple().to_string()).is_err());
        assert!(id(&Uuid::nil().to_string()).is_err());
        assert_eq!(
            serde_json::to_value(IntentView::Prepared {
                receipt_id: canonical,
                pieces: "6".to_owned(),
            })
            .unwrap(),
            json!({"state": "prepared", "receipt_id": canonical, "pieces": "6"})
        );
    }
}
