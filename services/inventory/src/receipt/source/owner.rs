//! Inventory's test-gated HTTP boundary for a new receipt or intent.
//!
//! The caller must select the stored plan and contact the configured trusted
//! Procurement owner. The preview owners must use the same issuer-to-actor
//! binding. A syntactically valid owner response is not proof of that binding.

use super::{MAX_SOURCE_BYTES, PurchaseLine, Reject, Selected, select};
use axum::{
    body::{Body, Bytes},
    http::{HeaderMap, HeaderValue, Request, Response, StatusCode, header},
};
use erp4us_identity::{Auth, ORGANIZATION_HEADER, Scope, TENANT_HEADER};
use http_body_util::{BodyExt, Limited};
use uuid::Uuid;

const POST_PERMISSION: &str = "inventory.purchase-receipt.post";
const UNAVAILABLE: StatusCode = StatusCode::SERVICE_UNAVAILABLE;

struct Receiver {
    identity: Scope,
    bearer: HeaderValue,
}

impl Receiver {
    fn verify(auth: &Auth, incoming: &HeaderMap) -> Result<Self, StatusCode> {
        let identity = crate::query::authorize(auth, incoming, POST_PERMISSION)?;
        for (header, id) in [
            (TENANT_HEADER, identity.tenant),
            (ORGANIZATION_HEADER, identity.organization),
        ] {
            if id.is_nil()
                || incoming.get(header).and_then(|value| value.to_str().ok())
                    != Some(id.to_string().as_str())
            {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        if identity.actor.is_nil() {
            return Err(StatusCode::FORBIDDEN);
        }
        let mut bearer = incoming
            .get(header::AUTHORIZATION)
            .ok_or(StatusCode::UNAUTHORIZED)?
            .clone();
        bearer.set_sensitive(true);
        Ok(Self { identity, bearer })
    }

    /// Only fixed, origin-form requests; no browser destination or headers are copied.
    fn request(&self, plan: Uuid) -> Result<Request<()>, StatusCode> {
        if plan.is_nil() {
            return Err(StatusCode::BAD_REQUEST);
        }
        Request::get(format!(
            "/v1/procurement/proposals/by-plan/{plan}/purchase-orders/receipt-source"
        ))
        .header(header::AUTHORIZATION, &self.bearer)
        .header(TENANT_HEADER, self.identity.tenant.to_string())
        .header(ORGANIZATION_HEADER, self.identity.organization.to_string())
        .header(header::ACCEPT, "application/json")
        .body(())
        .map_err(|_| UNAVAILABLE)
    }

    async fn response(
        &self,
        plan: Uuid,
        purchase: PurchaseLine,
        response: Response<Body>,
    ) -> Result<Selected, StatusCode> {
        let data = read_owner(response).await?;
        select(self.identity, plan, purchase, &data).map_err(|reject| match reject {
            Reject::NotFound => StatusCode::NOT_FOUND,
            _ => UNAVAILABLE,
        })
    }
}

async fn read_owner(response: Response<Body>) -> Result<Bytes, StatusCode> {
    match response.status() {
        StatusCode::OK
            if response.headers().get(header::CONTENT_TYPE)
                == Some(&HeaderValue::from_static("application/json")) =>
        {
            Limited::new(response.into_body(), MAX_SOURCE_BYTES)
                .collect()
                .await
                .map(|body| body.to_bytes())
                .map_err(|_| UNAVAILABLE)
        }
        status @ (StatusCode::FORBIDDEN | StatusCode::NOT_FOUND) => Err(status),
        // Inventory already authenticated this token; an owner-side 401 is
        // a failed credential binding or expiry during the call, not permission.
        _ => Err(UNAVAILABLE),
    }
}

#[path = "../../../../../tests/integration/receipt_identity.rs"]
mod receipt_identity;

#[cfg(test)]
mod tests {
    use super::{receipt_identity::*, *};
    use serde_json::json;
    use std::fs;

    const TENANT: Uuid = Uuid::from_u128(101);
    const ORG: Uuid = Uuid::from_u128(102);
    const ACTOR: Uuid = Uuid::from_u128(103);
    const PLAN: Uuid = Uuid::from_u128(104);
    const PAIR: PurchaseLine = PurchaseLine {
        order: Uuid::from_u128(105),
        line: Uuid::from_u128(106),
    };
    const SOURCE_PERMISSION: &str = "procurement.purchase-order.receipt-source.read";

    fn owner_auth(audience: &str, permissions: &[&str]) -> Auth {
        let folder = std::env::temp_dir().join(format!("inventory-owned-auth-{}", Uuid::new_v4()));
        fs::create_dir(&folder).unwrap();
        let policy = folder.join("policy.json");
        let jwks = folder.join("jwks.json");
        let config = json!({
            "issuer": ISSUER,
            "audience": audience,
            "principals": {(SUBJECT): {
                "actor_id": ACTOR,
                "grants": [{"tenant_id": TENANT, "organization_id": ORG, "permissions": permissions}]
            }}
        });
        fs::write(&policy, config.to_string()).unwrap();
        fs::write(&jwks, JWKS).unwrap();
        let auth = Auth::load(policy.to_str().unwrap(), jwks.to_str().unwrap());
        fs::remove_file(policy).unwrap();
        fs::remove_file(jwks).unwrap();
        fs::remove_dir(folder).unwrap();
        auth.unwrap()
    }

    fn headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static(BEARER));
        headers.insert(TENANT_HEADER, TENANT.to_string().parse().unwrap());
        headers.insert(ORGANIZATION_HEADER, ORG.to_string().parse().unwrap());
        headers
    }

    #[test]
    fn forwards_only_verified_context_and_the_original_token() {
        let inventory = owner_auth(AUDIENCE, &[POST_PERMISSION]);
        let procurement = owner_auth(AUDIENCE, &[SOURCE_PERMISSION]);
        let procurement_wrong_audience = owner_auth("other-service", &[SOURCE_PERMISSION]);
        let mut incoming = headers();
        incoming.insert("x-actor-id", HeaderValue::from_static("untrusted"));
        incoming.insert(header::HOST, HeaderValue::from_static("untrusted.invalid"));
        incoming.insert(header::COOKIE, HeaderValue::from_static("untrusted"));
        let receiver = Receiver::verify(&inventory, &incoming).unwrap();
        let request = receiver.request(PLAN).unwrap();
        assert_eq!(request.method(), "GET");
        assert_eq!(
            request.uri().to_string(),
            format!("/v1/procurement/proposals/by-plan/{PLAN}/purchase-orders/receipt-source")
        );
        assert_eq!(request.uri().authority(), None);
        let sent = request.headers();
        assert_eq!(sent.len(), 4);
        assert_eq!(sent.get(header::AUTHORIZATION).unwrap(), BEARER);
        assert!(sent.get(header::AUTHORIZATION).unwrap().is_sensitive());
        let identity = crate::query::authorize(&procurement, sent, SOURCE_PERMISSION).unwrap();
        assert_eq!(identity.actor, receiver.identity.actor);
        assert_eq!(identity.tenant, receiver.identity.tenant);
        assert_eq!(identity.organization, receiver.identity.organization);
        assert_eq!(
            crate::query::authorize(&procurement_wrong_audience, sent, SOURCE_PERMISSION)
                .unwrap_err(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            crate::query::authorize(&inventory, sent, SOURCE_PERMISSION).unwrap_err(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn does_not_prepare_an_owner_request_without_current_inventory_permission() {
        let inventory = owner_auth(AUDIENCE, &[POST_PERMISSION]);
        let no_post = owner_auth(AUDIENCE, &[SOURCE_PERMISSION]);
        let mut foreign = headers();
        foreign.insert(ORGANIZATION_HEADER, PLAN.to_string().parse().unwrap());
        let mut duplicate = headers();
        duplicate.append(header::AUTHORIZATION, HeaderValue::from_static(BEARER));
        let mut absent = headers();
        absent.remove(header::AUTHORIZATION);
        let mut noncanonical = headers();
        noncanonical.insert(TENANT_HEADER, TENANT.simple().to_string().parse().unwrap());
        let mut foreign_tenant = headers();
        foreign_tenant.insert(TENANT_HEADER, PLAN.to_string().parse().unwrap());
        for (policy, incoming, expected) in [
            (&no_post, headers(), StatusCode::FORBIDDEN),
            (&inventory, foreign, StatusCode::FORBIDDEN),
            (&inventory, foreign_tenant, StatusCode::FORBIDDEN),
            (&inventory, duplicate, StatusCode::UNAUTHORIZED),
            (&inventory, absent, StatusCode::UNAUTHORIZED),
            (&inventory, noncanonical, StatusCode::BAD_REQUEST),
        ] {
            assert_eq!(Receiver::verify(policy, &incoming).err(), Some(expected));
        }
    }

    fn response(status: StatusCode, content_type: &'static str, body: Vec<u8>) -> Response<Body> {
        Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(body))
            .unwrap()
    }

    #[tokio::test]
    async fn accepts_a_bounded_exact_source_and_fails_closed_on_owner_errors() {
        let auth = owner_auth(AUDIENCE, &[POST_PERMISSION]);
        let receiver = Receiver::verify(&auth, &headers()).unwrap();
        let document = json!({
            "plan_id": PLAN, "lines": [{
                "purchase_order_id": PAIR.order, "purchase_line_id": PAIR.line,
                "sku": "NM-ENDPLATE", "revision": null, "unit": "EA", "ordered_pieces": "12"
            }]
        });
        let data = serde_json::to_vec(&document).unwrap();
        let selected = receiver
            .response(
                PLAN,
                PAIR,
                response(StatusCode::OK, "application/json", data.clone()),
            )
            .await
            .unwrap();
        assert_eq!(selected.as_issued().actor, ACTOR);
        assert_eq!(selected.as_issued().purchase, PAIR);
        assert_eq!(selected.as_issued().ordered, 12);
        for (status, mime, body, expected) in [
            (StatusCode::OK, "text/html", data.clone(), UNAVAILABLE),
            (
                StatusCode::OK,
                "application/json",
                vec![0; MAX_SOURCE_BYTES + 1],
                UNAVAILABLE,
            ),
            (
                StatusCode::OK,
                "application/json",
                b"{}".to_vec(),
                UNAVAILABLE,
            ),
            (
                StatusCode::TEMPORARY_REDIRECT,
                "application/json",
                data,
                UNAVAILABLE,
            ),
            (
                StatusCode::UNAUTHORIZED,
                "application/json",
                vec![],
                UNAVAILABLE,
            ),
            (
                StatusCode::FORBIDDEN,
                "application/json",
                vec![],
                StatusCode::FORBIDDEN,
            ),
            (
                StatusCode::NOT_FOUND,
                "application/json",
                vec![],
                StatusCode::NOT_FOUND,
            ),
        ] {
            assert_eq!(
                receiver
                    .response(PLAN, PAIR, response(status, mime, body))
                    .await
                    .unwrap_err(),
                expected
            );
        }
    }
}
