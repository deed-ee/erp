use super::*;
use crate::{netting::Item, preview::Warehouse};
use erp4us_messaging::{Contract, Delivery, Envelope, stream};
use mongodb::{Client, bson, bson::Bson, options::DatabaseOptions};
use serde_json::{Value, json};

#[path = "../../../../../tests/integration/receipt_transport.rs"]
mod transport;

const SCOPE: erp4us_identity::Scope = erp4us_identity::Scope {
    tenant: Uuid::from_u128(1),
    organization: Uuid::from_u128(2),
    actor: Uuid::from_u128(3),
};
const ORDER: Uuid = Uuid::from_u128(4);
const LINE: Uuid = Uuid::from_u128(5);
const PLAN: Uuid = Uuid::from_u128(6);
const WAREHOUSE: Uuid = Uuid::from_u128(7);
const SOURCE_EVENT: Uuid = Uuid::from_u128(8);
const PURCHASE: PurchaseLine = PurchaseLine {
    order: Uuid::from_u128(9),
    line: Uuid::from_u128(10),
};

fn command(scope: erp4us_identity::Scope, id: u128, pieces: &str) -> Command {
    Command::new(scope, ORDER, LINE, PURCHASE, Uuid::from_u128(id), pieces).unwrap()
}

fn preparation(command: Command, acknowledged: Option<Uuid>) -> Preparation {
    Preparation::new(
        command.selection(),
        acknowledged,
        &command.pieces.to_string(),
    )
    .unwrap()
}

async fn prepare_new(
    store: &Store,
    command: Command,
    acknowledged: Option<Uuid>,
    destination: &StockKey,
) -> Command {
    let Prepare::Created(intent) = store
        .prepare(
            preparation(command, acknowledged),
            &source(command, &destination.item),
            destination,
        )
        .await
        .unwrap()
    else {
        panic!("expected a new prepared lot");
    };
    intent.command
}

fn stock(command: Command, sku: &str) -> StockKey {
    StockKey {
        scope: command.sales.scope,
        warehouse: WAREHOUSE,
        item: Item {
            sku: sku.to_owned(),
            revision: None,
        },
    }
}

fn source(command: Command, item: &Item) -> IssuedLine<'_> {
    IssuedLine {
        scope: command.sales.scope,
        actor: command.actor,
        plan: PLAN,
        purchase: command.purchase,
        item,
        unit: "EA",
        ordered: 12,
    }
}

fn plan_record(command: Command) -> Document {
    doc! {
        "_id": sales_key(command.sales), "schemaVersion": SCHEMA,
        "plan": {
            "id": PLAN.to_string(), "order_id": ORDER.to_string(), "line_id": LINE.to_string(),
            "warehouse_id": WAREHOUSE.to_string(), "source_event_id": SOURCE_EVENT.to_string(),
            "unit": "EA", "sku": "NM-1000", "product_revision": "A", "pieces": "10",
            "materials": [
                {"sku": "NM-ENDPLATE", "revision": Bson::Null, "requested": "20", "reserved": "8", "uncovered": "12"},
                {"sku": "NM-SCREW", "revision": Bson::Null, "requested": "12", "reserved": "0", "uncovered": "12"},
            ],
        },
        "handoff": {"pending": false, "envelope": "original handoff stays unchanged"},
    }
}

fn test_database(client: &Client, name: &str) -> Database {
    client.database_with_options(
        name,
        DatabaseOptions::builder()
            .write_concern(majority_write())
            .build(),
    )
}

async fn seed_waiting_endplates(db: &Database, command: Command, component: &Document) {
    db.collection::<Document>("material_plans")
        .insert_one(plan_record(command))
        .write_concern(majority_write())
        .await
        .unwrap();
    db.collection::<Document>("reservations")
        .insert_one(doc! {
            "_id": sales_key(command.sales), "schemaVersion": SCHEMA, "materialPlanId": PLAN.to_string(),
            "sourceEventId": SOURCE_EVENT.to_string(), "reserved": 0, "outstanding": 10,
            "stock": Warehouse { tenant: SCOPE.tenant, organization: SCOPE.organization, id: WAREHOUSE }
                .stock_key("NM-1000", Some("A")),
        })
        .write_concern(majority_write())
        .await
        .unwrap();
    db.collection::<Document>("balances")
        .insert_one(doc! {"_id": component, "schemaVersion": SCHEMA, "available": 0, "expected": 0})
        .write_concern(majority_write())
        .await
        .unwrap();
    db.collection::<Document>(LEDGER)
        .insert_one(doc! {
            "_id": {"stock": component, "kind": "preview-opening"},
            "schemaVersion": SCHEMA, "quantity": 8,
        })
        .write_concern(majority_write())
        .await
        .unwrap();
}

fn saved_fixture() -> Saved {
    let command = command(SCOPE, 11, "6");
    let stock = stock(command, "NM-ENDPLATE");
    let issued = source(command, &stock.item);
    let (plan, _) =
        records::original(&plan_record(command), command.sales, PLAN, &stock.item).unwrap();
    Saved {
        effect: post_new(
            command,
            &plan,
            &CurrentRow {
                key: plan.key.clone(),
                eligibility: Eligibility::Waiting,
                purchase: None,
                held: 8,
            },
            &issued,
            None,
            Some(&stock),
        )
        .unwrap(),
        posted_at: DateTime::from_millis(1_796_000_000_001),
        message_id: Uuid::from_u128(20),
    }
}

#[test]
fn bsons_round_trip_original_intent_time_and_scoped_binding() {
    let saved = saved_fixture();
    let command = saved.effect.command;
    let line = records::line_record(&saved.effect).unwrap();
    let encoded = records::receipt_record(
        &saved,
        &line,
        fact::encode(&saved, ActorKind::User).unwrap(),
    );
    let doc: Document = bson::from_slice(&bson::to_vec(&encoded).unwrap()).unwrap();
    assert_eq!(records::read_saved(command, &doc).unwrap(), saved);
    assert_eq!(
        doc.get_document("_id").unwrap(),
        &doc! {"tenantId": SCOPE.tenant.to_string(), "organizationId": SCOPE.organization.to_string(),
        "receiptId": command.receipt_id.to_string()}
    );
    let (state, allocation) = records::read_line(&line).unwrap();
    assert_eq!(state, saved.effect.line);
    assert_eq!(
        (
            allocation.allocated,
            allocation.open,
            allocation.held,
            allocation.shortage
        ),
        (6, 6, 14, 6)
    );
    assert_eq!(state.binding.row.original.held, 8);
    assert_eq!(state.binding.row.original.buy, 12);
    let key = line.get_document("_id").unwrap();
    assert_eq!(key.get("revision"), Some(&Bson::Null));
    assert_eq!(key.get_str("planId"), Ok(PLAN.to_string().as_str()));
    assert_eq!(purchase_index().options.unwrap().unique, Some(true));

    let movement = records::movement(&saved);
    let movement_key = movement.get_document("_id").unwrap();
    assert_eq!(movement_key.get_str("kind"), Ok("purchase-receipt"));
    assert_eq!(
        movement_key.get_document("stock").unwrap(),
        line.get_document("stockKey").unwrap()
    );
    assert_eq!(movement.get_i32("quantity"), Ok(6));
    assert_eq!(doc.get_i32("heldPieces"), Ok(6));
    assert_eq!(
        doc.get_datetime("postedAt"),
        movement.get_datetime("createdAt")
    );

    let mut different_actor = command;
    different_actor.actor = Uuid::from_u128(50);
    let mut changed_intent = command;
    changed_intent.sales.line = Uuid::from_u128(51);
    assert_eq!(
        records::read_saved(different_actor, &doc),
        Err(Reject::ActorMismatch)
    );
    assert_eq!(
        records::read_saved(changed_intent, &doc),
        Err(Reject::IntentChanged)
    );
    for foreign in [
        erp4us_identity::Scope {
            tenant: Uuid::from_u128(52),
            ..SCOPE
        },
        erp4us_identity::Scope {
            organization: Uuid::from_u128(53),
            ..SCOPE
        },
    ] {
        let other = Command::new(foreign, ORDER, LINE, PURCHASE, command.receipt_id, "6").unwrap();
        assert_ne!(receipt_key(other), receipt_key(command));
        assert_eq!(records::read_saved(other, &doc), Err(Reject::InvalidState));
    }
}

#[test]
fn bson_intent_round_trips_without_fabricating_a_receipt_or_replay_result() {
    let saved = saved_fixture();
    let command = saved.effect.command;
    let destination = saved.effect.line.binding.row.key.stock.clone();
    let intent = Intent {
        command,
        plan: PLAN,
        destination,
        acknowledged: None,
        confirmed: None,
    };
    let slot = records::head(command.selection(), command.receipt_id);
    assert_eq!(
        records::read_head(command.selection(), &slot),
        Ok(command.receipt_id)
    );
    let selected = slot.get_document("_id").unwrap();
    assert_eq!(
        selected,
        &doc! {
            "tenantId": SCOPE.tenant.to_string(), "organizationId": SCOPE.organization.to_string(),
            "orderId": ORDER.to_string(), "lineId": LINE.to_string(), "actorId": SCOPE.actor.to_string(),
            "purchaseOrderId": PURCHASE.order.to_string(), "purchaseLineId": PURCHASE.line.to_string(),
        }
    );
    let read = |record: &Document| {
        let round_trip = bson::from_slice(&bson::to_vec(record).unwrap()).unwrap();
        records::read_intent(
            command.sales.scope,
            command.actor,
            command.receipt_id,
            &round_trip,
        )
    };
    let prepared = records::prepared(&intent);
    assert!(!prepared.contains_key("outbox"));
    assert!(!prepared.contains_key("lineAtPosting"));
    assert_eq!(read(&prepared), Ok(intent));

    let mut confirmed = records::receipt_record(
        &saved,
        &records::line_record(&saved.effect).unwrap(),
        fact::encode(&saved, ActorKind::User).unwrap(),
    );
    // The later lot records its predecessor, even if the quantity is the same.
    let intent = Intent {
        acknowledged: Some(Uuid::from_u128(99)),
        ..read(&prepared).unwrap()
    };
    records::mark_confirmed(&intent, &mut confirmed);
    assert_eq!(
        read(&confirmed),
        Ok(Intent {
            confirmed: Some(Box::new(saved)),
            ..intent
        })
    );
    for (field, value) in [
        ("schemaVersion", Bson::Int32(0)),
        ("pieces", Bson::Int32(0)),
        ("pieces", Bson::String("6".into())),
        ("state", Bson::String("closed".into())),
        (
            "acknowledgedReceiptId",
            Bson::String(command.receipt_id.to_string()),
        ),
        ("planId", Bson::String(Uuid::nil().to_string())),
        ("outbox", Bson::Document(doc! {})),
    ] {
        let mut corrupt = prepared.clone();
        corrupt.insert(field, value);
        assert_eq!(read(&corrupt), Err(Reject::InvalidState));
    }
    let mut corrupt = confirmed;
    corrupt.insert("planId", Uuid::from_u128(99).to_string());
    assert_eq!(read(&corrupt), Err(Reject::InvalidState));
    let mut corrupt = prepared;
    corrupt
        .get_document_mut("selectionKey")
        .unwrap()
        .insert("actorId", Uuid::from_u128(99).to_string());
    assert_eq!(read(&corrupt), Err(Reject::InvalidState));
}

#[test]
fn source_row_lookup_requires_immutable_sales_pin_explicit_revision_and_ea() {
    let command = command(SCOPE, 11, "6");
    let item = stock(command, "NM-ENDPLATE").item;
    let plan = plan_record(command);
    let (_, waiting) = records::original(&plan, command.sales, PLAN, &item).unwrap();
    assert_eq!(waiting.get_document("_id"), Ok(&sales_key(command.sales)));
    assert_eq!(
        waiting.get_str("materialPlanId"),
        Ok(PLAN.to_string().as_str())
    );
    assert_eq!(waiting.get_i32("outstanding"), Ok(10));
    assert_eq!(
        waiting.get_str("sourceEventId"),
        Ok(SOURCE_EVENT.to_string().as_str())
    );
    let mut reservation = waiting;
    reservation.remove("cancellation");
    let (_, guard) = records::original(&plan, command.sales, PLAN, &item).unwrap();
    assert!(records::is_waiting(&reservation, &guard));
    reservation.insert("outstanding", vec![10]);
    assert!(!records::is_waiting(&reservation, &guard));
    reservation.insert("outstanding", 10);
    reservation.insert("cancellation", Bson::Null);
    assert!(!records::is_waiting(&reservation, &guard));
    assert_eq!(
        records::original(&plan, command.sales, Uuid::from_u128(99), &item),
        Err(Reject::InvalidSource)
    );
    assert_eq!(
        records::original(
            &plan,
            command.sales,
            PLAN,
            &Item {
                revision: Some("A".into()),
                ..item.clone()
            }
        ),
        Err(Reject::PlanIneligible)
    );
    for field in ["unit", "order_id", "line_id"] {
        let mut malformed = plan.clone();
        malformed
            .get_document_mut("plan")
            .unwrap()
            .insert(field, "wrong");
        assert_eq!(
            records::original(&malformed, command.sales, PLAN, &item),
            Err(Reject::InvalidState)
        );
    }
    let mut missing_revision = plan;
    missing_revision
        .get_document_mut("plan")
        .unwrap()
        .get_array_mut("materials")
        .unwrap()[0]
        .as_document_mut()
        .unwrap()
        .remove("revision");
    assert_eq!(
        records::original(&missing_revision, command.sales, PLAN, &item),
        Err(Reject::InvalidState)
    );
}

#[test]
fn pending_wire_fact_is_a_new_root_and_excludes_internal_fields() {
    let saved = saved_fixture();
    let example: Value = serde_json::from_str(include_str!(
        "../../../../../contracts/examples/nats/inventory-purchase-receipt-posted-v1.json"
    ))
    .unwrap();
    let reference: Envelope<fact::Posted> =
        serde_json::from_value(example["envelope"].clone()).unwrap();
    reference.validate().unwrap();

    let wire = fact::encode(&saved, ActorKind::Agent).unwrap();
    let event: Envelope<fact::Posted> = serde_json::from_str(&wire).unwrap();
    event.validate().unwrap();
    assert_eq!(event.message_id, saved.message_id);
    assert_eq!(event.correlation_id, event.message_id);
    assert_eq!(event.source_service, "erp4us.inventory");
    assert_eq!(event.tenant_id, Some(SCOPE.tenant));
    assert_eq!(event.organization_id, Some(SCOPE.organization));
    let actor = event.actor.unwrap();
    assert_eq!(actor.id, SCOPE.actor.to_string());
    assert_eq!(actor.kind, "agent");
    let wire: Value = serde_json::from_str(&wire).unwrap();
    assert!(wire.get("causationId").is_none());
    assert_eq!(
        wire["payload"],
        json!({
            "receiptId": saved.effect.command.receipt_id,
            "planId": PLAN, "purchaseOrderId": PURCHASE.order, "purchaseLineId": PURCHASE.line,
            "sku": "NM-ENDPLATE", "revision": null, "unit": "EA", "pieces": "6", "receiptSequence": 1
        })
    );
    let mut invalid = wire;
    invalid["payload"]
        .as_object_mut()
        .unwrap()
        .remove("revision");
    assert!(serde_json::from_value::<Envelope<fact::Posted>>(invalid).is_err());
}

#[tokio::test]
async fn posting_deadline_keeps_an_in_flight_task_alive_and_bounded() {
    let slots = Arc::new(Semaphore::new(1));
    let (release, work) = tokio::sync::oneshot::channel();
    let operation = async {
        work.await.map_err(|_| Failure::Unavailable)?;
        Ok::<_, Failure>(())
    };
    assert_eq!(
        bounded_post(&slots, Instant::now() + Duration::from_millis(5), operation).await,
        Err(Failure::Unavailable)
    );
    assert_eq!(slots.available_permits(), 0);
    assert_eq!(
        bounded_post(&slots, Instant::now() + Duration::from_secs(1), async {
            Ok::<_, Failure>(())
        })
        .await,
        Err(Failure::Unavailable)
    );
    release.send(()).unwrap();
    let _slot = timeout(Duration::from_secs(1), slots.acquire())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[ignore = "requires INVENTORY_RECEIPT_TEST_MONGO_URI for a dedicated disposable replica set"]
async fn mongo_competing_lots_and_rollback_keep_all_effects_atomic() {
    let uri = std::env::var("INVENTORY_RECEIPT_TEST_MONGO_URI").expect("dedicated test Mongo URI");
    let client = Client::with_uri_str(&uri)
        .await
        .expect("valid test Mongo settings");
    let db = test_database(
        &client,
        &format!("inventory_receipt_test_{}", Uuid::new_v4().simple()),
    );
    let store = Store::open(&db).await.unwrap();
    let plans = db.collection::<Document>("material_plans");
    let reservations = db.collection::<Document>("reservations");
    let balances = db.collection::<Document>("balances");
    let ledger = db.collection::<Document>(LEDGER);
    let receipts = db.collection::<Document>(RECEIPTS);
    let first = command(SCOPE, 11, "7");
    let competing = command(
        erp4us_identity::Scope {
            actor: Uuid::from_u128(12),
            ..SCOPE
        },
        12,
        "7",
    );
    let destination = stock(first, "NM-ENDPLATE");
    let component = stock_key(&destination);
    let screws = stock(first, "NM-SCREW");
    let issued = source(first, &destination.item);
    let historical = plan_record(first);
    let sales = sales_key(first.sales);
    seed_waiting_endplates(&db, first, &component).await;
    balances
        .insert_one(doc! {"_id": stock_key(&screws), "schemaVersion": SCHEMA, "available": 0, "expected": 0})
        .await
        .unwrap();
    assert_eq!(
        store.replay(first).await,
        Err(Failure::Rejected(Reject::NotFound))
    );
    assert_eq!(
        store.allocation(first.sales, PLAN, PURCHASE).await.unwrap(),
        None
    );
    let first = prepare_new(&store, first, None, &destination).await;
    let competing = prepare_new(&store, competing, None, &destination).await;
    let other_receiver_source = source(competing, &destination.item);
    let (a, b) = tokio::join!(
        store.post(first, &issued, &destination, ActorKind::User),
        store.post(
            competing,
            &other_receiver_source,
            &destination,
            ActorKind::User
        ),
    );
    let original = match (a, b) {
        (Ok(Post::Created(saved)), Err(Failure::Rejected(Reject::OverReceipt)))
        | (Err(Failure::Rejected(Reject::OverReceipt)), Ok(Post::Created(saved))) => saved,
        (a, b) => panic!("expected exactly one competing lot: {a:?}, {b:?}"),
    };
    let winner = original.effect.command;
    let issued = source(winner, &destination.item);
    let final_lot = Command {
        pieces: 5,
        ..winner
    };
    let another = PurchaseLine {
        order: PURCHASE.order,
        line: Uuid::from_u128(40),
    };
    let other_line = Command {
        purchase: another,
        ..final_lot
    };
    let other_issued = source(other_line, &destination.item);
    assert_eq!(
        store
            .prepare(preparation(other_line, None), &other_issued, &destination)
            .await,
        Err(Failure::Rejected(Reject::PlanIneligible))
    );
    let other_row = source(final_lot, &screws.item);
    assert_eq!(
        store
            .prepare(
                preparation(final_lot, Some(winner.receipt_id)),
                &other_row,
                &screws
            )
            .await,
        Err(Failure::Rejected(Reject::PlanIneligible))
    );
    assert_eq!(
        store
            .replay(Command {
                pieces: 6,
                ..winner
            })
            .await,
        Err(Failure::Rejected(Reject::IntentChanged))
    );
    assert_eq!(
        store
            .post(winner, &other_row, &screws, ActorKind::User)
            .await
            .unwrap(),
        Post::Replay(original)
    );
    let final_lot = prepare_new(&store, final_lot, Some(winner.receipt_id), &destination).await;
    let before = store
        .allocation(first.sales, PLAN, PURCHASE)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((before.sequence, before.allocated, before.open), (1, 7, 5));
    let before_balance = balances
        .find_one(doc! {"_id": &component})
        .await
        .unwrap()
        .unwrap();
    let before_reservation = reservations
        .find_one(doc! {"_id": &sales})
        .await
        .unwrap()
        .unwrap();

    // Make the final write fail after staged guards, allocation, receipt, and outbox.
    let conflict = doc! {"stock": &component, "kind": "purchase-receipt", "receiptId": final_lot.receipt_id.to_string()};
    ledger
        .insert_one(doc! {"_id": &conflict, "forcedTestConflict": true})
        .await
        .unwrap();
    assert_eq!(
        store
            .post(final_lot, &issued, &destination, ActorKind::User)
            .await,
        Err(Failure::Unavailable)
    );
    assert_eq!(store.replay(final_lot).await.unwrap(), None);
    assert_eq!(
        receipts
            .count_documents(doc! {"outbox.pending": true})
            .await
            .unwrap(),
        1
    );
    let pending = store.reopen(final_lot.selection()).await.unwrap().unwrap();
    assert_eq!(pending.command, final_lot);
    assert!(pending.confirmed.is_none());
    assert_eq!(
        store.allocation(first.sales, PLAN, PURCHASE).await.unwrap(),
        Some(before)
    );
    assert_eq!(
        balances.find_one(doc! {"_id": &component}).await.unwrap(),
        Some(before_balance)
    );
    assert_eq!(
        reservations.find_one(doc! {"_id": &sales}).await.unwrap(),
        Some(before_reservation)
    );
    assert_eq!(
        ledger
            .delete_one(doc! {"_id": &conflict, "forcedTestConflict": true})
            .await
            .unwrap()
            .deleted_count,
        1
    );

    // A once-verified owner/source and destination cannot override a deactivated reservation.
    reservations
        .update_one(doc! {"_id": &sales}, doc! {"$set": {"outstanding": 0}})
        .await
        .unwrap();
    assert_eq!(
        store
            .post(final_lot, &issued, &destination, ActorKind::User)
            .await,
        Err(Failure::Rejected(Reject::PlanIneligible))
    );
    reservations
        .update_one(doc! {"_id": &sales}, doc! {"$set": {"outstanding": 10}})
        .await
        .unwrap();
    assert!(matches!(
        store
            .post(final_lot, &issued, &destination, ActorKind::User)
            .await
            .unwrap(),
        Post::Created(_)
    ));

    let allocation = store
        .allocation(first.sales, PLAN, PURCHASE)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            allocation.sequence,
            allocation.allocated,
            allocation.open,
            allocation.held
        ),
        (2, 12, 0, 20)
    );
    let closed = command(SCOPE, 14, "1");
    reservations
        .update_one(doc! {"_id": &sales}, doc! {"$set": {"outstanding": 0}})
        .await
        .unwrap();
    assert_eq!(
        store
            .prepare(
                preparation(closed, Some(final_lot.receipt_id)),
                &issued,
                &destination,
            )
            .await,
        Err(Failure::Rejected(Reject::OverReceipt))
    );
    assert_eq!(
        store.replay(winner).await,
        Err(Failure::Rejected(Reject::IntentNotCurrent))
    );
    assert_eq!(
        store
            .replay(Command {
                actor: Uuid::from_u128(99),
                ..winner
            })
            .await,
        Err(Failure::Rejected(Reject::NotFound))
    );
    assert_eq!(
        store
            .replay(command(
                erp4us_identity::Scope {
                    organization: Uuid::from_u128(99),
                    ..SCOPE
                },
                winner.receipt_id.as_u128(),
                "7"
            ))
            .await,
        Err(Failure::Rejected(Reject::NotFound))
    );
    assert_eq!(
        balances
            .find_one(doc! {"_id": &component})
            .await
            .unwrap()
            .unwrap()
            .get_i32("available"),
        Ok(0)
    );
    assert_eq!(
        plans.find_one(doc! {"_id": &sales}).await.unwrap(),
        Some(historical)
    );
    assert_eq!(
        ledger
            .count_documents(doc! {"_id.kind": "purchase-receipt"})
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        receipts
            .count_documents(doc! {"outbox.pending": true})
            .await
            .unwrap(),
        2
    );
    db.drop().await.unwrap();
}

#[tokio::test]
#[ignore = "requires INVENTORY_RECEIPT_TEST_MONGO_URI for a dedicated disposable replica set"]
async fn mongo_recovers_the_same_lot_until_deliberately_advancing_the_current_intent() {
    let uri = std::env::var("INVENTORY_RECEIPT_TEST_MONGO_URI").expect("dedicated test Mongo URI");
    let client = Client::with_uri_str(&uri).await.unwrap();
    let db_name = format!("inventory_receipt_test_{}", Uuid::new_v4().simple());
    let db = test_database(&client, &db_name);
    let store = Store::open(&db).await.unwrap();
    let sample = command(SCOPE, 11, "6");
    let destination = stock(sample, "NM-ENDPLATE");
    let issued = source(sample, &destination.item);
    let component = stock_key(&destination);
    seed_waiting_endplates(&db, sample, &component).await;
    let balances = store.majority_read("balances");
    let reservations = store.majority_read("reservations");
    let receipts = store.majority_read(RECEIPTS);
    let ledger = store.majority_read(LEDGER);
    let reservation_key = doc! {"_id": sales_key(sample.sales)};
    let balance_key = doc! {"_id": &component};
    let original_balance = balances.find_one(balance_key.clone()).await.unwrap();
    let original_reservation = reservations
        .find_one(reservation_key.clone())
        .await
        .unwrap();

    let first_request = preparation(sample, None);
    assert_eq!(store.prepare_replay(first_request).await.unwrap(), None);
    let (a, b) = tokio::join!(
        store.prepare(first_request, &issued, &destination),
        store.prepare(first_request, &issued, &destination),
    );
    let first = match (a.unwrap(), b.unwrap()) {
        (Prepare::Created(original), Prepare::Replay(retry))
        | (Prepare::Replay(retry), Prepare::Created(original)) => {
            assert_eq!(original, retry);
            original.command
        }
        other => panic!("expected one durable ID and its replay: {other:?}"),
    };
    assert_eq!(receipts.count_documents(doc! {}).await.unwrap(), 1);
    assert_eq!(
        receipts
            .count_documents(doc! {"outbox": {"$exists": true}})
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        ledger
            .count_documents(doc! {"_id.kind": "purchase-receipt"})
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        balances.find_one(balance_key).await.unwrap(),
        original_balance
    );
    assert_eq!(
        reservations
            .find_one(reservation_key.clone())
            .await
            .unwrap(),
        original_reservation
    );
    assert_eq!(
        store.allocation(first.sales, PLAN, PURCHASE).await.unwrap(),
        None
    );

    // Lose the browser's reply and reconstruct the store with a new client.
    let reconnected = Client::with_uri_str(&uri).await.unwrap();
    let store = Store::open(&test_database(&reconnected, &db_name))
        .await
        .unwrap();
    let pending = store.reopen(sample.selection()).await.unwrap().unwrap();
    assert_eq!(pending.command, first);
    assert!(pending.confirmed.is_none());
    assert_eq!(
        store.prepare_replay(first_request).await.unwrap(),
        Some(pending)
    );
    assert_eq!(
        store
            .prepare_replay(preparation(Command { pieces: 5, ..first }, None))
            .await,
        Err(Failure::Rejected(Reject::IntentChanged)),
    );
    let later_request = preparation(first, Some(first.receipt_id));
    assert_eq!(
        store.prepare_replay(later_request).await,
        Err(Failure::Rejected(Reject::IntentNotCurrent)),
    );
    for foreign in [
        Command {
            actor: Uuid::from_u128(91),
            ..first
        },
        command(
            erp4us_identity::Scope {
                tenant: Uuid::from_u128(92),
                ..SCOPE
            },
            first.receipt_id.as_u128(),
            "6",
        ),
        command(
            erp4us_identity::Scope {
                organization: Uuid::from_u128(93),
                ..SCOPE
            },
            first.receipt_id.as_u128(),
            "6",
        ),
    ] {
        assert_eq!(store.reopen(foreign.selection()).await.unwrap(), None);
        assert_eq!(
            store.replay(foreign).await,
            Err(Failure::Rejected(Reject::NotFound))
        );
        assert_eq!(
            store
                .post(
                    foreign,
                    &source(foreign, &destination.item),
                    &destination,
                    ActorKind::User
                )
                .await,
            Err(Failure::Rejected(Reject::NotFound)),
        );
    }
    let changed_line = Command {
        purchase: PurchaseLine {
            line: Uuid::from_u128(94),
            ..PURCHASE
        },
        ..first
    };
    assert_eq!(
        store.replay(changed_line).await,
        Err(Failure::Rejected(Reject::IntentChanged))
    );
    let different_item = stock(first, "NM-SCREW");
    assert_eq!(
        store
            .post(
                first,
                &source(first, &different_item.item),
                &different_item,
                ActorKind::User
            )
            .await,
        Err(Failure::Rejected(Reject::InvalidSource)),
    );
    assert_eq!(store.replay(first).await.unwrap(), None);

    let (a, b) = tokio::join!(
        store.post(first, &issued, &destination, ActorKind::User),
        store.post(first, &issued, &destination, ActorKind::User),
    );
    let original = match (a.unwrap(), b.unwrap()) {
        (Post::Created(saved), Post::Replay(retry))
        | (Post::Replay(retry), Post::Created(saved)) => {
            assert_eq!(saved, retry);
            retry
        }
        other => panic!("expected one confirmation and its original replay: {other:?}"),
    };
    assert_eq!(original.effect.plan_held, 14);
    assert_eq!(original.effect.line_open, 6);
    // Reopening and exact replay are independent of the waiting state and source.
    reservations
        .update_one(reservation_key.clone(), doc! {"$set": {"outstanding": 0}})
        .await
        .unwrap();
    assert_eq!(store.replay(first).await.unwrap().as_ref(), Some(&original));
    assert_eq!(
        store
            .reopen(first.selection())
            .await
            .unwrap()
            .unwrap()
            .confirmed
            .as_deref(),
        Some(&original),
    );
    assert_eq!(
        store
            .prepare_replay(first_request)
            .await
            .unwrap()
            .unwrap()
            .confirmed
            .as_deref(),
        Some(&original),
    );
    assert_eq!(
        store.prepare(later_request, &issued, &destination).await,
        Err(Failure::Rejected(Reject::PlanIneligible)),
    );
    assert_eq!(store.replay(first).await.unwrap().as_ref(), Some(&original));
    reservations
        .update_one(reservation_key, doc! {"$set": {"outstanding": 10}})
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        store.prepare(later_request, &issued, &destination),
        store.prepare(later_request, &issued, &destination),
    );
    let second = match (a.unwrap(), b.unwrap()) {
        (Prepare::Created(original), Prepare::Replay(retry))
        | (Prepare::Replay(retry), Prepare::Created(original)) => {
            assert_eq!(original, retry);
            original.command
        }
        other => panic!("expected one distinct later lot and its replay: {other:?}"),
    };
    assert_ne!(second.receipt_id, first.receipt_id);
    assert_eq!(second.pieces, first.pieces);
    assert_eq!(
        store.prepare_replay(first_request).await,
        Err(Failure::Rejected(Reject::IntentNotCurrent))
    );
    let (late, new) = tokio::join!(
        store.post(first, &issued, &destination, ActorKind::User),
        store.post(second, &issued, &destination, ActorKind::User),
    );
    assert_eq!(late, Err(Failure::Rejected(Reject::IntentNotCurrent)));
    let Post::Created(final_receipt) = new.unwrap() else {
        panic!("expected the distinct lot");
    };
    assert_eq!(final_receipt.effect.line.sequence, 2);
    assert_eq!(final_receipt.effect.line_open, 0);
    assert_eq!(final_receipt.effect.plan_held, 20);
    let historical = receipts
        .find_one(doc! {"_id": receipt_key(first)})
        .await
        .unwrap()
        .unwrap();
    assert_eq!(records::read_saved(first, &historical).unwrap(), original);
    assert_eq!(
        receipts
            .count_documents(doc! {"outbox.pending": true})
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        ledger
            .count_documents(doc! {"_id.kind": "purchase-receipt"})
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        store
            .majority_read(SLOTS)
            .count_documents(doc! {})
            .await
            .unwrap(),
        1
    );
    db.drop().await.unwrap();
}

#[tokio::test]
#[ignore = "requires disposable loopback Mongo replica set and empty JetStream; run before Procurement transport test"]
async fn receipt_transport_publish_committed_inventory_outbox() {
    let (db, jetstream) = transport::connect("inventory").await;
    let ops = jetstream.operations();
    assert_eq!(
        ops.query_account().await.unwrap().streams,
        0,
        "the proof must start with an empty, disposable JetStream"
    );
    let store = Store::open(&db).await.unwrap();
    let first = command(SCOPE, 11, "6");
    let destination = stock(first, "NM-ENDPLATE");
    let issued = source(first, &destination.item);
    seed_waiting_endplates(&db, first, &stock_key(&destination)).await;
    let mut committed = Vec::new();
    let mut acknowledged = None;
    for lot in [11, 12] {
        let Prepare::Created(intent) = store
            .prepare_with_candidate(
                preparation(first, acknowledged),
                Uuid::from_u128(lot),
                &issued,
                &destination,
            )
            .await
            .unwrap()
        else {
            panic!("expected a distinct prepared lot");
        };
        let command = intent.command;
        let Post::Created(saved) = store
            .post(command, &issued, &destination, ActorKind::User)
            .await
            .unwrap()
        else {
            panic!("expected a new committed lot");
        };
        let row = store
            .majority_read(RECEIPTS)
            .find_one(doc! {
                "_id": receipt_key(command), "outbox.pending": true,
                "outbox.messageId": saved.message_id.to_string(),
            })
            .projection(doc! {"_id": 0, "outbox.envelope": 1})
            .await
            .unwrap()
            .unwrap();
        let body = row
            .get_document("outbox")
            .unwrap()
            .get_str("envelope")
            .unwrap();
        let event: Envelope<fact::Posted> = serde_json::from_str(body).unwrap();
        assert_eq!(event.message_id, saved.message_id);
        // JetStream::publish must send the same body, not a reconstructed fixture.
        assert_eq!(serde_json::to_string(&event).unwrap(), body);
        committed.push(event);
        acknowledged = Some(command.receipt_id);
    }
    db.drop().await.unwrap();

    let Delivery::JetStream(name) = fact::Posted::DELIVERY else {
        panic!("the owner contract requires JetStream");
    };
    ops.create_stream(stream::Config {
        name: name.into(),
        description: Some(transport::STREAM_MARKER.into()),
        subjects: vec![committed[0].subject().unwrap()],
        storage: stream::StorageType::Memory,
        max_messages: 3,
        max_age: Duration::from_secs(900),
        max_message_size: 8 * 1024,
        discard: stream::DiscardPolicy::New,
        ..Default::default()
    })
    .await
    .unwrap();
    // Deliver the second physical lot first. Separate test processes share only this stream.
    for (sequence, event) in (1..).zip(committed.iter().rev()) {
        let ack = jetstream.publish(event).await.unwrap();
        assert_eq!(ack.sequence, sequence);
        assert!(!ack.duplicate);
    }
}
