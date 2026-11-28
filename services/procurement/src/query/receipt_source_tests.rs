use super::*;
use serde_json::{Value, json};
use std::fs;

#[path = "../../../../tests/integration/receipt_identity.rs"]
mod receipt_identity;
use receipt_identity::{AUDIENCE, BEARER, ISSUER, JWKS, SUBJECT};

const TENANT: &str = "ddcae139-c1e8-47d2-8b48-349cb6f30ce5";
const ORG: &str = "3e793fb5-75a7-45f6-8f66-45a58fcec674";
const PLAN: &str = "8af68084-19d6-4e31-8203-f9f4708b9bee";
const ACTOR: Uuid = Uuid::from_u128(11);
fn auth(actor: Uuid, audience: &str, permissions: &[&str]) -> Auth {
    let grants = |tenant: &str, organization: &str, permissions: &[&str]| -> Value {
        json!({"tenant_id": tenant, "organization_id": organization, "permissions": permissions})
    };
    let config = json!({
        "issuer": ISSUER,
        "audience": audience,
        "principals": {
            (SUBJECT): {"actor_id": actor, "grants": [
                grants(TENANT, ORG, permissions),
                grants(PLAN, ORG, &[READ_PURCHASE_PERMISSION]),
                grants(TENANT, PLAN, &[READ_PURCHASE_PERMISSION]),
            ]},
        },
    });
    let directory = std::env::temp_dir().join(format!("procurement-auth-{}", Uuid::new_v4()));
    fs::create_dir(&directory).unwrap();
    let config_path = directory.join("policy.json");
    let jwks_path = directory.join("jwks.json");
    fs::write(&config_path, config.to_string()).unwrap();
    fs::write(&jwks_path, JWKS).unwrap();
    let result = Auth::load(config_path.to_str().unwrap(), jwks_path.to_str().unwrap());
    fs::remove_dir_all(directory).unwrap();
    result.unwrap()
}

fn headers(tenant: &'static str, org: &'static str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, HeaderValue::from_static(BEARER));
    headers.insert(TENANT_HEADER, HeaderValue::from_static(tenant));
    headers.insert(ORGANIZATION_HEADER, HeaderValue::from_static(org));
    headers
}

#[test]
fn receipt_source_permission_is_separate_and_exactly_scoped() {
    let local = headers(TENANT, ORG);
    let receiver = auth(ACTOR, AUDIENCE, &[READ_RECEIPT_SOURCE_PERMISSION]);
    let (identity, plan) = receipt_source_request(&receiver, &local, PLAN).unwrap();
    assert_eq!(identity.actor, ACTOR);
    assert_eq!(identity.tenant.to_string(), TENANT);
    assert_eq!(identity.organization.to_string(), ORG);
    assert_eq!(plan.to_string(), PLAN);
    assert_eq!(
        purchase_identity(&receiver, &local, false).unwrap_err(),
        StatusCode::FORBIDDEN,
    );
    for outside in [headers(PLAN, ORG), headers(TENANT, PLAN)] {
        // Commercial access in a second scope does not imply receipt access.
        assert!(purchase_identity(&receiver, &outside, false).is_ok());
        assert_eq!(
            receipt_source_request(&receiver, &outside, PLAN).unwrap_err(),
            StatusCode::FORBIDDEN,
        );
    }
    let commercial = auth(
        ACTOR,
        AUDIENCE,
        &[
            READ_PERMISSION,
            READ_PURCHASE_PERMISSION,
            ISSUE_PURCHASE_PERMISSION,
            "procurement.approval.department",
            "procurement.approval.budget",
        ],
    );
    assert!(purchase_identity(&commercial, &local, true).is_ok());
    assert_eq!(
        receipt_source_request(&commercial, &local, PLAN).unwrap_err(),
        StatusCode::FORBIDDEN,
    );
    let nil_actor = auth(Uuid::nil(), AUDIENCE, &[READ_RECEIPT_SOURCE_PERMISSION]);
    assert_eq!(
        receipt_source_request(&nil_actor, &local, PLAN).unwrap_err(),
        StatusCode::FORBIDDEN,
    );
}

#[test]
fn receipt_source_requires_verified_identity_and_canonical_selectors() {
    let receiver = auth(ACTOR, AUDIENCE, &[READ_RECEIPT_SOURCE_PERMISSION]);
    let wrong_audience = auth(
        ACTOR,
        "another-test-service",
        &[READ_RECEIPT_SOURCE_PERMISSION],
    );
    let local = headers(TENANT, ORG);
    assert_eq!(
        receipt_source_request(&wrong_audience, &local, PLAN).unwrap_err(),
        StatusCode::UNAUTHORIZED,
    );
    let mut missing_token = local.clone();
    missing_token.remove(header::AUTHORIZATION);
    assert_eq!(
        receipt_source_request(&receiver, &missing_token, PLAN).unwrap_err(),
        StatusCode::UNAUTHORIZED,
    );
    for bad in [
        headers("ddcae139c1e847d28b48349cb6f30ce5", ORG),
        headers(TENANT, "3E793FB5-75A7-45F6-8F66-45A58FCEC674"),
        headers("00000000-0000-0000-0000-000000000000", ORG),
        headers(TENANT, "00000000-0000-0000-0000-000000000000"),
    ] {
        assert_eq!(
            receipt_source_request(&receiver, &bad, PLAN).unwrap_err(),
            StatusCode::BAD_REQUEST,
        );
    }
    for bad in [
        "".to_owned(),
        PLAN.to_uppercase(),
        PLAN.replace('-', ""),
        Uuid::nil().to_string(),
    ] {
        assert_eq!(
            receipt_source_request(&receiver, &local, &bad).unwrap_err(),
            StatusCode::BAD_REQUEST,
        );
    }
}
