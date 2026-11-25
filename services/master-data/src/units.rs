use crate::{
    domain::Scope,
    http::{DEMO_SCOPE, problem, unavailable},
    store::{InsertError, Store},
};
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterUnit {
    code: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Unit {
    #[serde(rename = "_id")]
    #[serde(with = "uuid::serde::hyphenated")]
    id: Uuid,
    #[serde(with = "uuid::serde::hyphenated")]
    tenant_id: Uuid,
    #[serde(with = "uuid::serde::hyphenated")]
    organization_id: Uuid,
    #[serde(with = "uuid::serde::hyphenated")]
    created_by: Uuid,
    schema_version: u32,
    revision: u32,
    code: String,
}

impl Unit {
    fn register(scope: Scope, command: RegisterUnit) -> Result<Self, &'static str> {
        let code = command.code;
        if !(1..=16).contains(&code.len())
            || !code.as_bytes()[0].is_ascii_uppercase()
            || !code
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(
                "Unit code must start with an uppercase ASCII letter and contain 1-16 uppercase letters, digits or hyphens",
            );
        }
        Ok(Self {
            id: Uuid::new_v4(),
            tenant_id: scope.tenant,
            organization_id: scope.organization,
            created_by: scope.actor,
            schema_version: 1,
            revision: 1,
            code,
        })
    }

    fn view(self) -> UnitView {
        UnitView {
            id: self.id,
            tenant_id: self.tenant_id,
            organization_id: self.organization_id,
            created_by: self.created_by,
            schema_version: self.schema_version,
            revision: self.revision,
            code: self.code,
        }
    }
}

#[derive(Serialize)]
struct UnitView {
    id: Uuid,
    tenant_id: Uuid,
    organization_id: Uuid,
    created_by: Uuid,
    schema_version: u32,
    revision: u32,
    code: String,
}

pub(crate) async fn register(
    State(store): State<Store>,
    body: Result<Json<RegisterUnit>, JsonRejection>,
) -> Response {
    let Json(command) = match body {
        Ok(body) => body,
        Err(rejection) => return problem(rejection.status(), "Invalid JSON request"),
    };
    let unit = match Unit::register(DEMO_SCOPE, command) {
        Ok(unit) => unit,
        Err(detail) => return problem(StatusCode::UNPROCESSABLE_ENTITY, detail),
    };
    match store.insert_unit(&unit).await {
        Ok(()) => (
            StatusCode::CREATED,
            [(header::LOCATION, format!("/v1/units/{}", unit.id))],
            Json(unit.view()),
        )
            .into_response(),
        Err(InsertError::Conflict) => problem(StatusCode::CONFLICT, "Unit code already registered"),
        Err(InsertError::Database(error)) => unavailable("register_unit", error),
    }
}

pub(crate) async fn find(State(store): State<Store>, Path(id): Path<String>) -> Response {
    let Ok(id) = Uuid::parse_str(&id) else {
        return problem(StatusCode::BAD_REQUEST, "Invalid unit ID");
    };
    match store.get_unit(DEMO_SCOPE, id).await {
        Ok(Some(unit)) => Json(unit.view()).into_response(),
        Ok(None) => problem(StatusCode::NOT_FOUND, "Unit not found"),
        Err(error) => unavailable("find_unit", error),
    }
}
