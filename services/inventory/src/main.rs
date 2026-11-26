//! Trusted, one-shot fixture adapter for the first Inventory use case.
use mongodb::{
    Client, ClientSession, Collection, Database,
    bson::{DateTime, Document, doc},
    error::Error,
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    io::{self, Read},
    process::ExitCode,
};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConfirmedLine {
    contract: Contract,
    order_id: Uuid,
    line_id: Uuid,
    warehouse_id: Uuid,
    article_id: Uuid,
    unit: Unit,
    quantity: i32,
}
#[derive(Deserialize)]
enum Contract {
    #[serde(rename = "confirmed-stock-line.v1")]
    V1,
}
#[derive(Deserialize)]
enum Unit {
    #[serde(rename = "EA")]
    Each,
}
#[derive(Serialize)]
struct Allocation {
    reserved: i32,
    outstanding: i32,
}

async fn reserve(
    db: Database,
    scope: Document,
    line: ConfirmedLine,
) -> mongodb::error::Result<Allocation> {
    let mut line_key = scope.clone();
    line_key.extend(doc! {"orderId":line.order_id.to_string(), "lineId":line.line_id.to_string()});
    let mut stock_key = scope;
    stock_key.extend(doc! {"warehouseId":line.warehouse_id.to_string(), "articleId":line.article_id.to_string(), "unit":"EA"});
    let stock = db.collection::<Document>("balances");
    let reservations = db.collection::<Document>("reservations");
    let mut session = db.client().start_session().await?;
    session
        .start_transaction()
        .and_run2(async |session| {
            allocate(
                session,
                &stock,
                &reservations,
                &stock_key,
                &line_key,
                line.quantity,
            )
            .await
        })
        .await
}

async fn allocate(
    session: &mut ClientSession,
    stock: &Collection<Document>,
    reservations: &Collection<Document>,
    stock_key: &Document,
    line_key: &Document,
    requested: i32,
) -> mongodb::error::Result<Allocation> {
    let line_filter = doc! { "_id": line_key };
    if let Some(old) = reservations
        .find_one(line_filter)
        .session(&mut *session)
        .await?
    {
        if old.get_document("stock") != Ok(stock_key) || old.get_i32("requested") != Ok(requested) {
            return Err(Error::custom("conflicting confirmed line"));
        }
        return Ok(Allocation {
            reserved: old.get_i32("reserved").map_err(Error::custom)?,
            outstanding: old.get_i32("outstanding").map_err(Error::custom)?,
        });
    }
    let stock_filter = doc! { "_id": stock_key };
    let balance = stock
        .find_one(stock_filter.clone())
        .session(&mut *session)
        .await?;
    let available = balance
        .as_ref()
        .map(|b| b.get_i32("available"))
        .transpose()
        .map_err(Error::custom)?
        .unwrap_or(0);
    if available < 0 {
        return Err(Error::custom("invalid stock balance"));
    }
    let reserved = requested.min(available);
    let outstanding = requested - reserved;
    if reserved > 0 {
        stock
            .update_one(stock_filter, doc! { "$inc": {"available": -reserved} })
            .session(&mut *session)
            .await?;
    }
    let record = doc! {
        "_id": line_key, "stock": stock_key, "requested": requested,
        "reserved": reserved, "outstanding": outstanding, "schemaVersion": 1,
        "createdAt": DateTime::now(), "actor": "inventory.local-fixture",
    };
    reservations.insert_one(record).session(session).await?;
    Ok(Allocation {
        reserved,
        outstanding,
    })
}

async fn run() -> Result<(), &'static str> {
    let id = |key| {
        env::var(key)
            .ok()
            .and_then(|s| Uuid::parse_str(&s).ok())
            .map(|id| id.to_string())
            .ok_or("missing Inventory scope")
    };
    let scope = doc! {"tenantId":id("INVENTORY_TENANT_ID")?, "organizationId":id("INVENTORY_ORGANIZATION_ID")?};
    let uri = env::var("INVENTORY_MONGO_URI").map_err(|_| "missing Inventory connection")?;
    let mut bytes = Vec::new();
    io::stdin()
        .take(8193)
        .read_to_end(&mut bytes)
        .map_err(|_| "cannot read fixture")?;
    if bytes.len() > 8192 {
        return Err("fixture too large");
    }
    let line: ConfirmedLine =
        serde_json::from_slice(&bytes).map_err(|_| "invalid confirmed stock line")?;
    let (Contract::V1, Unit::Each) = (&line.contract, &line.unit);
    if line.quantity <= 0 {
        return Err("quantity must be positive");
    }
    let db = Client::with_uri_str(uri)
        .await
        .map_err(|_| "invalid Inventory connection")?
        .database("inventory");
    let result = reserve(db, scope, line)
        .await
        .map_err(|_| "reservation not acknowledged; check the fixture and retry the same line")?;
    println!(
        "{}",
        serde_json::to_string(&result).map_err(|_| "cannot encode allocation")?
    );
    Ok(())
}
#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
