//! Test-gated Inventory persistence for the proposed waiting-plan receipt.
//!
//! A scoped receipt ID is reserved at preparation. A separate fixed-size slot
//! points to the current receipt for the verified actor and selection. Advancing
//! the slot does not rewrite the predecessor's immutable result. The Inventory
//! movement ledger remains the physical record; no part of a lot becomes free stock.
//! Sales order/line selects the immutable plan in this preview; the assigned
//! plan and destination are then pinned on the intent itself.
//!
//! Future callers must authorize before replay, check it before any owner lookup,
//! verify the owner tuple with the same actor/scope, and verify the destination. This
//! adapter has no owner, broker or HTTP connection. A future release/consumption path
//! must serialize on the same reservation/line and account for these additional holds.

use super::{
    Binding, Command, CurrentRow, Eligibility, IssuedLine, LineState, Organization, OriginalBuyRow,
    PlanRow, PlanRowKey, Preparation, PrepareAction, PurchaseLine, ReceiptEffect, Reject,
    SalesLine, Selection, StockKey, check_confirmation, check_replay, post_new, prepare_action,
};
use mongodb::{
    ClientSession, Collection, Database, IndexModel,
    bson::{DateTime, Document, doc},
    error::{
        Error as MongoError, ErrorKind, TRANSIENT_TRANSACTION_ERROR,
        UNKNOWN_TRANSACTION_COMMIT_RESULT, WriteFailure,
    },
    options::{
        Acknowledgment, CollectionOptions, IndexOptions, ReadConcern, ReadPreference,
        SelectionCriteria, WriteConcern,
    },
};
use serde::{Deserialize, Serialize};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::{
    sync::Semaphore,
    time::{Instant, sleep, timeout, timeout_at},
};
use uuid::Uuid;

mod fact;
mod records;
#[cfg(test)]
mod tests;

pub(super) use fact::ActorKind;
use records::{
    SCHEMA, purchase_key, receipt_key, receipt_ref, row_key, sales_key, selection_key, stock_key,
};

const LINES: &str = "purchase_receipt_lines";
const RECEIPTS: &str = "purchase_receipts";
const SLOTS: &str = "purchase_receipt_intent_slots";
const LEDGER: &str = "stock_movements";
const DB_LIMIT: Duration = Duration::from_secs(5);
const COMMIT_LIMIT: Duration = Duration::from_secs(2);
const MAX_ATTEMPTS: u32 = 5;
const MAX_POSTS: usize = 16;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Allocation {
    pub(super) sequence: i32,
    pub(super) allocated: i32,
    pub(super) open: i32,
    pub(super) held: i32,
    pub(super) shortage: i32,
}

impl Allocation {
    pub(super) fn from_effect(effect: &ReceiptEffect) -> Self {
        Self {
            sequence: effect.line.sequence,
            allocated: effect.line.allocated,
            open: effect.line_open,
            held: effect.plan_held,
            shortage: effect.plan_remaining_shortage,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Saved {
    pub(super) effect: ReceiptEffect,
    pub(super) posted_at: DateTime,
    message_id: Uuid,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Intent {
    pub(super) command: Command,
    acknowledged: Option<Uuid>,
    plan: Uuid,
    destination: StockKey,
    pub(super) confirmed: Option<Box<Saved>>,
}

impl Intent {
    fn state(&self) -> (Command, Option<Uuid>, bool) {
        (self.command, self.acknowledged, self.confirmed.is_some())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Prepare {
    Created(Intent),
    Replay(Intent),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Post {
    Created(Saved),
    Replay(Saved),
}

#[derive(Clone, Copy)]
enum Write {
    Prepare(Preparation, Uuid),
    Confirm(Command, ActorKind),
}

enum Written {
    Intent(Prepare),
    Receipt(Post),
}

struct CheckedLot {
    effect: ReceiptEffect,
    waiting_filter: Document,
    stock_filter: Document,
    previous_sequence: Option<i32>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Failure {
    Rejected(Reject),
    /// Includes unknown commit outcome. Retry the same preparation or confirmation.
    Unavailable,
}

enum Step {
    Rejected(Reject),
    Database(MongoError),
}

impl From<Reject> for Step {
    fn from(error: Reject) -> Self {
        Self::Rejected(error)
    }
}

impl From<MongoError> for Step {
    fn from(error: MongoError) -> Self {
        Self::Database(error)
    }
}

fn primary() -> SelectionCriteria {
    SelectionCriteria::ReadPreference(ReadPreference::Primary)
}

fn majority_write() -> WriteConcern {
    WriteConcern::builder()
        .w(Acknowledgment::Majority)
        .w_timeout(COMMIT_LIMIT)
        .build()
}

fn purchase_index() -> IndexModel {
    IndexModel::builder()
        .keys(doc! {"purchaseKey": 1})
        .options(
            IndexOptions::builder()
                .name("one_row_per_scoped_purchase_line".to_owned())
                .unique(true)
                .build(),
        )
        .build()
}

#[derive(Clone)]
pub(super) struct Store {
    db: Database,
    posting_slots: Arc<Semaphore>,
}

impl Store {
    /// Mongo's `_id` indexes cover the reserved receipt ID, current actor
    /// slot and scoped plan row. A purchase tuple cannot bind to two rows.
    pub(super) async fn open(db: &Database) -> Result<Self, Failure> {
        timeout(DB_LIMIT, async {
            for name in [LINES, RECEIPTS, SLOTS, LEDGER] {
                match db
                    .create_collection(name)
                    .write_concern(majority_write())
                    .await
                {
                    // Existing collections already have their builtin _id indexes.
                    Err(error) if matches!(&*error.kind, ErrorKind::Command(e) if e.code == 48) => {
                    }
                    result => result.map_err(|_| Failure::Unavailable)?,
                }
            }
            db.collection::<Document>(LINES)
                .create_index(purchase_index())
                .write_concern(majority_write())
                .await
                .map_err(|_| Failure::Unavailable)?;
            Ok(Self {
                db: db.clone(),
                posting_slots: Arc::new(Semaphore::new(MAX_POSTS)),
            })
        })
        .await
        .map_err(|_| Failure::Unavailable)?
    }

    fn majority_read(&self, name: &str) -> Collection<Document> {
        self.db.collection_with_options(
            name,
            CollectionOptions::builder()
                .read_concern(ReadConcern::majority())
                .selection_criteria(primary())
                .build(),
        )
    }

    /// Check before any plan, destination or Procurement lookup. Only a
    /// current prepared receipt returns `None` and permits source verification.
    /// The caller must already have verified current receiver authorization.
    pub(super) async fn replay(&self, command: Command) -> Result<Option<Saved>, Failure> {
        self.read_snapshot(async |session| {
            Ok(self
                .confirmation_target(session, command)
                .await?
                .confirmed
                .map(|saved| *saved))
        })
        .await
    }

    pub(super) async fn reopen(&self, selection: Selection) -> Result<Option<Intent>, Failure> {
        self.read_snapshot(async |session| self.current(session, selection).await)
            .await
    }

    /// Called before looking up any owner source to create a new intent.
    pub(super) async fn prepare_replay(
        &self,
        request: Preparation,
    ) -> Result<Option<Intent>, Failure> {
        self.read_snapshot(async |session| {
            let current = self.current(session, request.selection).await?;
            match prepare_action(request, current.as_ref().map(Intent::state))? {
                PrepareAction::Start => Ok(None),
                PrepareAction::Replay => Ok(current),
            }
        })
        .await
    }

    /// Resolve the Inventory-owned plan for the exact, already authorized Sales line.
    /// Confirmation still rechecks the original plan, mapping and line in its write transaction.
    pub(super) async fn plan_destination(&self, sales: SalesLine) -> Result<(Uuid, Uuid), Failure> {
        timeout(DB_LIMIT, async {
            let document = self
                .majority_read("material_plans")
                .find_one(doc! {"_id": sales_key(sales)})
                .max_time(DB_LIMIT)
                .await
                .map_err(|_| Failure::Unavailable)?
                .ok_or(Failure::Rejected(Reject::NotFound))?;
            records::plan_header(&document, sales)
                .map(|(_, plan, warehouse)| (plan, warehouse))
                .map_err(Failure::Rejected)
        })
        .await
        .map_err(|_| Failure::Unavailable)?
    }

    async fn read_snapshot<T>(
        &self,
        read: impl AsyncFnOnce(&mut ClientSession) -> Result<T, Step>,
    ) -> Result<T, Failure> {
        timeout(DB_LIMIT, async {
            // Both reads see the same majority-committed snapshot, without
            // opening a write transaction or needing to resolve a commit.
            let mut session = self
                .db
                .client()
                .start_session()
                .snapshot(true)
                .await
                .map_err(|_| Failure::Unavailable)?;
            read(&mut session).await.map_err(|step| match step {
                Step::Rejected(reject) => Failure::Rejected(reject),
                Step::Database(_) => Failure::Unavailable,
            })
        })
        .await
        .map_err(|_| Failure::Unavailable)?
    }

    async fn session(&self) -> Result<ClientSession, Failure> {
        self.db
            .client()
            .start_session()
            .await
            .map_err(|_| Failure::Unavailable)
    }

    async fn head(
        &self,
        session: &mut ClientSession,
        selection: Selection,
    ) -> Result<Option<Uuid>, Step> {
        Ok(self
            .db
            .collection::<Document>(SLOTS)
            .find_one(doc! {"_id": selection_key(selection)})
            .selection_criteria(primary())
            .session(session)
            .await?
            .map(|document| records::read_head(selection, &document))
            .transpose()?)
    }

    async fn intent(
        &self,
        session: &mut ClientSession,
        scope: Organization,
        actor: Uuid,
        receipt_id: Uuid,
    ) -> Result<Option<Intent>, Step> {
        Ok(self
            .db
            .collection::<Document>(RECEIPTS)
            .find_one(doc! {"_id": receipt_ref(scope, receipt_id), "actorId": actor.to_string()})
            .selection_criteria(primary())
            .session(session)
            .await?
            .map(|document| records::read_intent(scope, actor, receipt_id, &document))
            .transpose()?)
    }

    async fn current(
        &self,
        session: &mut ClientSession,
        selection: Selection,
    ) -> Result<Option<Intent>, Step> {
        let Some(head) = self.head(session, selection).await? else {
            return Ok(None);
        };
        let intent = self
            .intent(session, selection.sales.scope, selection.actor, head)
            .await?
            .ok_or(Reject::InvalidState)?;
        if intent.command.selection() != selection {
            return Err(Reject::InvalidState.into());
        }
        Ok(Some(intent))
    }

    async fn confirmation_target(
        &self,
        session: &mut ClientSession,
        command: Command,
    ) -> Result<Intent, Step> {
        // Query by verified actor and scope before comparing or exposing any
        // state. A guessed receipt ID owned by someone else is absent.
        let intent = self
            .intent(
                session,
                command.sales.scope,
                command.actor,
                command.receipt_id,
            )
            .await?
            .ok_or(Reject::NotFound)?;
        let head = self
            .head(session, intent.command.selection())
            .await?
            .ok_or(Reject::InvalidState)?;
        check_confirmation(command, intent.command, head)?;
        Ok(intent)
    }

    /// Read only an existing allocation for the exact authorized Sales and purchase tuple.
    #[cfg(test)]
    async fn allocation(
        &self,
        sales: SalesLine,
        plan: Uuid,
        purchase: PurchaseLine,
    ) -> Result<Option<Allocation>, Failure> {
        self.allocation_snapshot(sales, plan, purchase)
            .await
            .map(|snapshot| snapshot.map(|(_, allocation)| allocation))
    }

    pub(super) async fn allocation_snapshot(
        &self,
        sales: SalesLine,
        plan: Uuid,
        purchase: PurchaseLine,
    ) -> Result<Option<(Binding, Allocation)>, Failure> {
        timeout(DB_LIMIT, async {
            let Some(document) = self
                .majority_read(LINES)
                .find_one(doc! {
                    "purchaseKey": purchase_key(sales.scope, plan, purchase),
                    "reservationKey": sales_key(sales),
                })
                .max_time(DB_LIMIT)
                .await
                .map_err(|_| Failure::Unavailable)?
            else {
                return Ok(None);
            };
            let (line, allocation) = records::read_line(&document).map_err(Failure::Rejected)?;
            if line.binding.row.key.sales != sales
                || line.binding.row.key.plan != plan
                || line.binding.purchase != purchase
            {
                return Err(Failure::Rejected(Reject::InvalidState));
            }
            Ok(Some((line.binding, allocation)))
        })
        .await
        .map_err(|_| Failure::Unavailable)?
    }

    pub(super) async fn post(
        &self,
        command: Command,
        issued: &IssuedLine<'_>,
        destination: &StockKey,
        actor_kind: ActorKind,
    ) -> Result<Post, Failure> {
        match self
            .write(Write::Confirm(command, actor_kind), issued, destination)
            .await?
        {
            Written::Receipt(value) => Ok(value),
            Written::Intent(_) => Err(Failure::Unavailable),
        }
    }

    pub(super) async fn prepare(
        &self,
        request: Preparation,
        issued: &IssuedLine<'_>,
        destination: &StockKey,
    ) -> Result<Prepare, Failure> {
        self.prepare_with_candidate(request, Uuid::new_v4(), issued, destination)
            .await
    }

    // Separate the server-assigned candidate from command data. The transport
    // fixture pins this value to the partner test's expected lot ID.
    async fn prepare_with_candidate(
        &self,
        request: Preparation,
        candidate: Uuid,
        issued: &IssuedLine<'_>,
        destination: &StockKey,
    ) -> Result<Prepare, Failure> {
        match self
            .write(Write::Prepare(request, candidate), issued, destination)
            .await?
        {
            Written::Intent(value) => Ok(value),
            Written::Receipt(_) => Err(Failure::Unavailable),
        }
    }

    async fn write(
        &self,
        write: Write,
        issued: &IssuedLine<'_>,
        destination: &StockKey,
    ) -> Result<Written, Failure> {
        let store = self.clone();
        let destination = destination.clone();
        let IssuedLine {
            scope,
            actor,
            plan,
            purchase,
            item,
            unit,
            ordered,
        } = *issued;
        let item = item.clone();
        let unit = unit.to_owned();
        let deadline = Instant::now() + DB_LIMIT;
        bounded_post(&self.posting_slots, deadline, async move {
            let issued = IssuedLine {
                scope,
                actor,
                plan,
                purchase,
                item: &item,
                unit: &unit,
                ordered,
            };
            store.transact(write, &issued, &destination, deadline).await
        })
        .await
    }

    async fn transact(
        &self,
        write: Write,
        issued: &IssuedLine<'_>,
        destination: &StockKey,
        deadline: Instant,
    ) -> Result<Written, Failure> {
        let mut session = self.session().await?;
        for attempt in 0..MAX_ATTEMPTS {
            if Instant::now() >= deadline {
                return Err(Failure::Unavailable);
            }
            begin_snapshot(&mut session).await?;

            let result = match write {
                Write::Prepare(request, candidate) => self
                    .apply_prepare(&mut session, request, candidate, issued, destination)
                    .await
                    .map(Written::Intent),
                Write::Confirm(command, actor_kind) => self
                    .apply_confirm(&mut session, command, issued, destination, actor_kind)
                    .await
                    .map(Written::Receipt),
            };
            match result {
                Ok(post) => {
                    if confirm_commit(&mut session).await? {
                        return Ok(post);
                    }
                }
                Err(step) => {
                    // No commit was sent. Abort before retrying a contended transaction.
                    session
                        .abort_transaction()
                        .await
                        .map_err(|_| Failure::Unavailable)?;
                    match step {
                        Step::Rejected(reject) => return Err(Failure::Rejected(reject)),
                        Step::Database(error) if transient(&error) || duplicate(&error) => (),
                        Step::Database(_) => return Err(Failure::Unavailable),
                    }
                }
            }
            retry_pause(attempt).await;
        }
        Err(Failure::Unavailable)
    }

    async fn apply_prepare(
        &self,
        session: &mut ClientSession,
        request: Preparation,
        candidate: Uuid,
        issued: &IssuedLine<'_>,
        destination: &StockKey,
    ) -> Result<Prepare, Step> {
        let prior = self.current(session, request.selection).await?;
        if prepare_action(request, prior.as_ref().map(Intent::state))? == PrepareAction::Replay {
            return Ok(Prepare::Replay(prior.ok_or(Reject::InvalidState)?));
        }
        if prior
            .as_ref()
            .is_some_and(|intent| intent.plan != issued.plan)
        {
            return Err(Reject::InvalidSource.into());
        }
        if candidate.is_nil() {
            return Err(Reject::InvalidState.into());
        }
        let command = request.assign(candidate);
        // This checks one snapshot, not a reservation: another lot may consume
        // the remaining room. Only confirmation takes the receipt write guards.
        let checked = self
            .check_lot(session, command, issued, destination)
            .await?;
        let row = checked.effect.line.binding.row.key;
        let intent = Intent {
            command,
            acknowledged: request.acknowledged,
            plan: row.plan,
            destination: row.stock,
            confirmed: None,
        };
        self.db
            .collection::<Document>(RECEIPTS)
            .insert_one(records::prepared(&intent))
            .session(&mut *session)
            .await?;
        let slots = self.db.collection::<Document>(SLOTS);
        let head = records::head(request.selection, command.receipt_id);
        if let Some(prior) = prior {
            let changed = slots
                .replace_one(
                    doc! {"_id": selection_key(request.selection), "receiptId": prior.command.receipt_id.to_string()},
                    head,
                )
                .session(session)
                .await?;
            if changed.modified_count != 1 {
                return Err(Reject::InvalidState.into());
            }
        } else {
            slots.insert_one(head).session(session).await?;
        }
        Ok(Prepare::Created(intent))
    }

    async fn check_lot(
        &self,
        session: &mut ClientSession,
        command: Command,
        issued: &IssuedLine<'_>,
        destination: &StockKey,
    ) -> Result<CheckedLot, Step> {
        if issued.scope != command.sales.scope || issued.purchase != command.purchase {
            return Err(Reject::NotFound.into());
        }
        if issued.actor != command.actor || issued.plan.is_nil() || !issued.item.valid() {
            return Err(Reject::InvalidSource.into());
        }

        let saved_plan = self
            .db
            .collection::<Document>("material_plans")
            .find_one(doc! {"_id": sales_key(command.sales)})
            .session(&mut *session)
            .await?
            .ok_or(Reject::NotFound)?;
        let (plan, waiting_filter) =
            records::original(&saved_plan, command.sales, issued.plan, issued.item)?;
        let row = row_key(&plan.key);
        let purchase = purchase_key(command.sales.scope, plan.key.plan, command.purchase);
        let lines = self.db.collection::<Document>(LINES);
        let old = lines
            .find_one(doc! {"_id": &row})
            .session(&mut *session)
            .await?
            .map(|document| records::read_line(&document))
            .transpose()?;
        if old
            .as_ref()
            .is_some_and(|(line, _)| line.binding.row != plan)
        {
            return Err(Reject::InvalidState.into());
        }
        let exact = old
            .as_ref()
            .map(|(line, _)| line)
            .filter(|line| line.binding.purchase == command.purchase);
        if exact.is_none()
            && lines
                .find_one(doc! {"purchaseKey": &purchase})
                .projection(doc! {"_id": 1})
                .session(&mut *session)
                .await?
                .is_some()
        {
            return Err(Reject::PlanIneligible.into());
        }

        let reservations = self.db.collection::<Document>("reservations");
        let waiting = reservations
            .find_one(waiting_filter.clone())
            .session(&mut *session)
            .await?
            .is_some_and(|row| records::is_waiting(&row, &waiting_filter));
        let current = CurrentRow {
            key: plan.key.clone(),
            eligibility: if waiting {
                Eligibility::Waiting
            } else {
                Eligibility::Incompatible
            },
            purchase: old.as_ref().map(|(line, _)| line.binding.purchase),
            held: old
                .as_ref()
                .map_or(plan.original.held, |(_, allocation)| allocation.held),
        };
        let balances = self.db.collection::<Document>("balances");
        let stock = stock_key(&plan.key.stock);
        let stock_filter = doc! {"_id": &stock, "schemaVersion": SCHEMA, "available": {"$gte": 0}};
        let mapped = destination == &plan.key.stock
            && balances
                .find_one(stock_filter.clone())
                .session(&mut *session)
                .await?
                .is_some_and(|row| {
                    row.get_document("_id") == Ok(&stock)
                        && row.get_i32("schemaVersion") == Ok(SCHEMA)
                        && row
                            .get_i32("available")
                            .is_ok_and(|available| available >= 0)
                });
        let effect = post_new(
            command,
            &plan,
            &current,
            issued,
            exact,
            mapped.then_some(destination),
        )?;
        Ok(CheckedLot {
            effect,
            waiting_filter,
            stock_filter,
            previous_sequence: old.map(|(_, allocation)| allocation.sequence),
        })
    }

    async fn apply_confirm(
        &self,
        session: &mut ClientSession,
        command: Command,
        issued: &IssuedLine<'_>,
        destination: &StockKey,
        actor_kind: ActorKind,
    ) -> Result<Post, Step> {
        let intent = self.confirmation_target(session, command).await?;
        if let Some(saved) = intent.confirmed {
            return Ok(Post::Replay(*saved));
        }
        if issued.plan != intent.plan || *issued.item != intent.destination.item {
            return Err(Reject::InvalidSource.into());
        }
        if *destination != intent.destination {
            return Err(Reject::UnverifiedDestination.into());
        }
        let CheckedLot {
            effect,
            waiting_filter,
            stock_filter,
            previous_sequence,
        } = self
            .check_lot(session, command, issued, destination)
            .await?;
        let message_id = Uuid::new_v4();
        let saved = Saved {
            effect,
            posted_at: DateTime::now(),
            message_id,
        };
        let next = records::line_record(&saved.effect)?;
        let mut receipt = records::receipt_record(&saved, &next, fact::encode(&saved, actor_kind)?);
        records::mark_confirmed(&intent, &mut receipt);

        let row = &saved.effect.line.binding.row.key;
        let row_id = row_key(row);
        let purchase = purchase_key(command.sales.scope, row.plan, command.purchase);
        let reservations = self.db.collection::<Document>("reservations");
        let balances = self.db.collection::<Document>("balances");
        let lines = self.db.collection::<Document>(LINES);

        // Real writes (not matched no-ops) guard against deletion/changes after
        // the snapshot. These tokens do not change demand or available stock.
        guard(&reservations, waiting_filter, message_id, session).await?;
        guard(&balances, stock_filter, message_id, session).await?;
        if let Some(sequence) = previous_sequence {
            let changed = lines
                .update_one(
                    doc! {"_id": row_id, "purchaseKey": purchase, "allocation.sequence": sequence},
                    doc! {"$set": {"allocation": next.get_document("allocation").map_err(|_| Reject::InvalidState)?}},
                )
                .session(&mut *session)
                .await?;
            if changed.modified_count != 1 {
                return Err(Reject::InvalidState.into());
            }
        } else {
            lines.insert_one(next).session(&mut *session).await?;
        }
        let changed = self
            .db
            .collection::<Document>(RECEIPTS)
            .replace_one(
                doc! {"_id": receipt_key(command), "actorId": command.actor.to_string(), "state": "prepared"},
                receipt,
            )
            .session(&mut *session)
            .await?;
        if changed.modified_count != 1 {
            return Err(Reject::InvalidState.into());
        }
        self.db
            .collection::<Document>(LEDGER)
            .insert_one(records::movement(&saved))
            .session(session)
            .await?;
        Ok(Post::Created(saved))
    }
}

async fn begin_snapshot(session: &mut ClientSession) -> Result<(), Failure> {
    session
        .start_transaction()
        .read_concern(ReadConcern::snapshot())
        .selection_criteria(primary())
        .write_concern(majority_write())
        .max_commit_time(COMMIT_LIMIT)
        .await
        .map_err(|_| Failure::Unavailable)
}

async fn bounded_post<T: Send + 'static>(
    slots: &Arc<Semaphore>,
    deadline: Instant,
    operation: impl Future<Output = Result<T, Failure>> + Send + 'static,
) -> Result<T, Failure> {
    let slot = slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| Failure::Unavailable)?;
    // Timing out or dropping the caller does not cancel the Mongo future,
    // including an in-flight commit. The task retains its slot until it
    // finishes; a client with an uncertain outcome retries the same intent.
    let task = tokio::spawn(async move {
        let _slot = slot;
        operation.await
    });
    timeout_at(deadline, task)
        .await
        .map_err(|_| Failure::Unavailable)?
        .map_err(|_| Failure::Unavailable)?
}

async fn guard(
    collection: &Collection<Document>,
    filter: Document,
    message_id: Uuid,
    session: &mut ClientSession,
) -> Result<(), Step> {
    let result = collection
        .update_one(
            filter,
            doc! {"$set": {"purchaseReceiptGuard": message_id.to_string()}},
        )
        .session(session)
        .await?;
    if result.modified_count != 1 {
        return Err(Reject::InvalidState.into());
    }
    Ok(())
}

fn transient(error: &MongoError) -> bool {
    error.contains_label(TRANSIENT_TRANSACTION_ERROR)
}

fn duplicate(error: &MongoError) -> bool {
    matches!(&*error.kind, ErrorKind::Write(WriteFailure::WriteError(e)) if e.code == 11000)
}

async fn retry_pause(attempt: u32) {
    sleep(Duration::from_millis(10 << attempt)).await;
}

/// An unknown result may have committed; only resend commit on this session.
/// `false` permits a fresh attempt only for a definite transient abort.
async fn confirm_commit(session: &mut ClientSession) -> Result<bool, Failure> {
    let mut uncertain = false;
    for attempt in 0..MAX_ATTEMPTS {
        match session.commit_transaction().await {
            Ok(()) => return Ok(true),
            Err(error) if error.contains_label(UNKNOWN_TRANSACTION_COMMIT_RESULT) => {
                uncertain = true;
                retry_pause(attempt).await;
            }
            Err(error) if !uncertain && transient(&error) => return Ok(false),
            Err(_) => return Err(Failure::Unavailable),
        }
    }
    Err(Failure::Unavailable)
}
