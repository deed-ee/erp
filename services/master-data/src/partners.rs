use crate::{
    domain::Scope,
    http::{problem, unavailable},
    store::Store,
};
use axum::{
    Extension, Json,
    extract::{Path, State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use mongodb::{
    Collection, Database, IndexModel,
    bson::doc,
    error::{ErrorKind, WriteFailure},
    options::IndexOptions,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Role {
    Customer,
    Supplier,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterPartner {
    code: String,
    name: String,
    roles: Vec<Role>,
}

#[derive(Deserialize, Serialize)]
struct Partner {
    #[serde(rename = "_id", with = "uuid::serde::hyphenated")]
    id: Uuid,
    #[serde(with = "uuid::serde::hyphenated")]
    tenant_id: Uuid,
    #[serde(with = "uuid::serde::hyphenated")]
    organization_id: Uuid,
    #[serde(with = "uuid::serde::hyphenated")]
    created_by: Uuid,
    schema_version: u32,
    revision: u32,
    #[serde(flatten)]
    details: RegisterPartner,
}

#[derive(Serialize)]
struct PartnerView {
    id: Uuid,
    tenant_id: Uuid,
    organization_id: Uuid,
    created_by: Uuid,
    schema_version: u32,
    revision: u32,
    #[serde(flatten)]
    details: RegisterPartner,
}

impl Partner {
    fn new(scope: Scope, details: RegisterPartner) -> Result<Self, &'static str> {
        if !(1..=32).contains(&details.code.len())
            || !details
                .code
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err("Partner code must have 1-32 uppercase ASCII letters, digits or hyphens");
        }
        if !(1..=120).contains(&details.name.chars().count())
            || details.name.trim().is_empty()
            || details.name.chars().any(char::is_control)
        {
            return Err(
                "Partner name must have 1-120 characters, without controls, and cannot be blank",
            );
        }
        match details.roles.as_slice() {
            [_] => {}
            [first, second] if first != second => {}
            _ => return Err("Choose customer, supplier, or both without duplicates"),
        }
        Ok(Self {
            id: Uuid::new_v4(),
            tenant_id: scope.tenant,
            organization_id: scope.organization,
            created_by: scope.actor,
            revision: 1,
            schema_version: 1,
            details,
        })
    }

    fn view(self) -> PartnerView {
        PartnerView {
            id: self.id,
            tenant_id: self.tenant_id,
            organization_id: self.organization_id,
            created_by: self.created_by,
            schema_version: self.schema_version,
            revision: self.revision,
            details: self.details,
        }
    }
}

fn collection(database: &Database) -> Collection<Partner> {
    database.collection("partners")
}

pub(crate) async fn ensure_index(database: &Database) -> mongodb::error::Result<()> {
    collection(database)
        .create_index(
            IndexModel::builder()
                .keys(doc! {"tenant_id": 1, "organization_id": 1, "code": 1})
                .options(
                    IndexOptions::builder()
                        .name("partner_scope_code_v1".to_owned())
                        .unique(true)
                        .build(),
                )
                .build(),
        )
        .await?;
    Ok(())
}

pub(crate) async fn register(
    State(store): State<Store>,
    Extension(scope): Extension<Scope>,
    body: Result<Json<RegisterPartner>, JsonRejection>,
) -> Response {
    let Json(command) = match body {
        Ok(body) => body,
        Err(rejection) => return problem(rejection.status(), "Invalid JSON request"),
    };
    let partner = match Partner::new(scope, command) {
        Ok(partner) => partner,
        Err(detail) => return problem(StatusCode::UNPROCESSABLE_ENTITY, detail),
    };
    match collection(store.database()).insert_one(&partner).await {
        Ok(_) => (
            StatusCode::CREATED,
            [(header::LOCATION, format!("/v1/partners/{}", partner.id))],
            Json(partner.view()),
        )
            .into_response(),
        Err(error) => match error.kind.as_ref() {
            ErrorKind::Write(WriteFailure::WriteError(e)) if e.code == 11000 => {
                problem(StatusCode::CONFLICT, "Partner code already registered")
            }
            _ => unavailable("register_partner", error),
        },
    }
}

pub(crate) async fn find(
    State(store): State<Store>,
    Extension(scope): Extension<Scope>,
    Path(id): Path<String>,
) -> Response {
    let Ok(id) = Uuid::parse_str(&id) else {
        return problem(StatusCode::BAD_REQUEST, "Invalid partner ID");
    };
    match collection(store.database())
        .find_one(doc! {
            "_id": id.to_string(),
            "tenant_id": scope.tenant.to_string(),
            "organization_id": scope.organization.to_string(),
        })
        .await
    {
        Ok(Some(partner)) => Json(partner.view()).into_response(),
        Ok(None) => problem(StatusCode::NOT_FOUND, "Partner not found"),
        Err(error) => unavailable("find_partner", error),
    }
}
