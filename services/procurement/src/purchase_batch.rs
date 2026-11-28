//! Formation of a local purchase issuance record; no supplier or inventory effects.

use crate::{
    drafts::Draft,
    material_plan::{BuyRow, canonical_id, revision, sku},
    terms::{Cents, Currency, MAX_LINES, Saved, Supplier},
};
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Failure {
    InvalidIntent,
    TermsMismatch,
    InvalidSnapshot,
    InvalidLineKey,
    DuplicateLineKey,
    InconsistentSupplier,
    InconsistentTotal,
    Overflow,
    InvalidAllocatedId,
}

/// Explicit command identity and reviewed versions. Not evidence of approval or permission.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    #[serde(with = "canonical_id")]
    pub command_id: Uuid,
    pub terms_revision: i32,
    #[serde(with = "canonical_id")]
    pub policy_id: Uuid,
    pub policy_revision: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Origin {
    pub intent: Intent,
    #[serde(with = "canonical_id")]
    pub actor_id: Uuid,
    pub issued_at: DateTime,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Batch {
    #[serde(with = "canonical_id")]
    batch_id: Uuid,
    origin: Origin,
    #[serde(with = "canonical_id")]
    proposal_id: Uuid,
    #[serde(with = "canonical_id")]
    plan_id: Uuid,
    #[serde(with = "canonical_id")]
    source_order_id: Uuid,
    #[serde(with = "canonical_id")]
    source_line_id: Uuid,
    currency: Currency,
    total: Cents,
    orders: Vec<Order>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Order {
    #[serde(with = "canonical_id")]
    order_id: Uuid,
    supplier: Supplier,
    total: Cents,
    lines: Vec<Line>,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Line {
    #[serde(with = "canonical_id")]
    line_id: Uuid,
    #[serde(flatten)]
    row: BuyRow,
    unit_price: Cents,
    line_total: Cents,
}

struct Group<'a> {
    supplier: &'a Supplier,
    total: Cents,
    lines: Vec<(&'a BuyRow, Cents, Cents)>,
}

impl Batch {
    /// Call only for a new issuance after verifying scope, issuer separation and all
    /// stages against the same policy, saved terms, and whole EUR total. The caller
    /// owns that check and replay; it must not use a client total or re-form a stored batch.
    pub(super) fn form(
        draft: &Draft,
        saved: &Saved,
        origin: Origin,
        approved_total_cents: u64,
        mut allocate_id: impl FnMut() -> Uuid,
    ) -> Result<Self, Failure> {
        let terms = &saved.terms;
        let plan = &draft.plan;
        if origin.intent.command_id.is_nil()
            || origin.actor_id.is_nil()
            || origin.intent.policy_id.is_nil()
            || origin.intent.policy_revision < 1
            || origin.intent.terms_revision < 1
        {
            return Err(Failure::InvalidIntent);
        }
        if origin.intent.terms_revision != terms.revision {
            return Err(Failure::TermsMismatch);
        }
        if !saved.has_valid_revision()
            || saved.actor_id.is_nil()
            || terms.plan_id != plan.plan_id
            || terms.proposal_id != draft.proposal_id
            || [plan.plan_id, plan.order_id, plan.line_id, draft.proposal_id]
                .iter()
                .any(Uuid::is_nil)
        {
            return Err(Failure::InvalidSnapshot);
        }
        if terms.total().as_u64() != approved_total_cents {
            return Err(Failure::InconsistentTotal);
        }
        let lines = terms.issuance_lines();
        if !(1..=MAX_LINES).contains(&lines.len()) || lines.len() != plan.buy_rows.len() {
            return Err(Failure::InvalidSnapshot);
        }
        // The saved snapshot is normally sorted. Canonicalize independently of storage order.
        let mut lines: Vec<_> = lines.collect();
        lines.sort_unstable_by_key(|(row, ..)| row.key());
        if !lines.iter().all(|(row, ..)| {
            let (item, rev) = row.key();
            sku(item) && rev.is_none_or(revision)
        }) {
            return Err(Failure::InvalidLineKey);
        }
        if lines
            .windows(2)
            .any(|pair| pair[0].0.key() == pair[1].0.key())
        {
            return Err(Failure::DuplicateLineKey);
        }

        let mut planned: Vec<_> = plan.buy_rows.iter().collect();
        planned.sort_unstable_by_key(|row| row.key());
        let mut groups = BTreeMap::new();
        for ((row, supplier, unit_price, line_total), planned) in lines.into_iter().zip(planned) {
            if row != planned {
                return Err(Failure::InvalidSnapshot);
            }
            let pieces = row.whole_pieces().ok_or(Failure::InvalidSnapshot)?;
            if unit_price
                .checked_mul(pieces)
                .map_err(|_| Failure::Overflow)?
                != line_total
            {
                return Err(Failure::InconsistentTotal);
            }
            if !valid_supplier(supplier) {
                return Err(Failure::InconsistentSupplier);
            }
            let group = groups.entry(supplier.id).or_insert_with(|| Group {
                supplier,
                total: Cents::ZERO,
                lines: Vec::new(),
            });
            if group.supplier != supplier {
                return Err(Failure::InconsistentSupplier);
            }
            group.total = group
                .total
                .checked_add(line_total)
                .map_err(|_| Failure::Overflow)?;
            group.lines.push((row, unit_price, line_total));
        }
        let total = groups.values().try_fold(Cents::ZERO, |total, group| {
            total
                .checked_add(group.total)
                .map_err(|_| Failure::Overflow)
        })?;
        if total != terms.total() {
            return Err(Failure::InconsistentTotal);
        }

        // Allocate only after verifying the whole snapshot. Refuse rather than reuse an ID.
        let mut ids = HashSet::new();
        let mut fresh_id = || {
            let id = allocate_id();
            if id.is_nil() || !ids.insert(id) {
                return Err(Failure::InvalidAllocatedId);
            }
            Ok(id)
        };
        let batch_id = fresh_id()?;
        let mut orders = Vec::with_capacity(groups.len());
        for group in groups.into_values() {
            let order_id = fresh_id()?;
            let lines = group
                .lines
                .into_iter()
                .map(|(row, unit_price, line_total)| {
                    Ok(Line {
                        line_id: fresh_id()?,
                        row: row.clone(),
                        unit_price,
                        line_total,
                    })
                })
                .collect::<Result<_, _>>()?;
            orders.push(Order {
                order_id,
                supplier: group.supplier.clone(),
                total: group.total,
                lines,
            });
        }
        Ok(Self {
            batch_id,
            origin,
            proposal_id: terms.proposal_id,
            plan_id: terms.plan_id,
            source_order_id: plan.order_id,
            source_line_id: plan.line_id,
            currency: Currency::Eur,
            total,
            orders,
        })
    }

    pub(super) fn origin(&self) -> Origin {
        self.origin
    }

    pub(super) fn batch_id(&self) -> Uuid {
        self.batch_id
    }

    pub(super) fn plan_id(&self) -> Uuid {
        self.plan_id
    }
}

// Check the historical snapshot shape, not whether the supplier is still active.
fn valid_supplier(supplier: &Supplier) -> bool {
    !supplier.id.is_nil()
        && (1..=32).contains(&supplier.code.len())
        && supplier
            .code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
        && (1..=120).contains(&supplier.name.chars().count())
        && !supplier.name.trim().is_empty()
        && !supplier.name.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{material_plan::tests::read, terms::tests::command};
    use serde_json::{Value, json};
    use std::collections::HashMap;

    const APPROVED: u64 = 188_240;
    const FIXTURE: &str =
        include_str!("../../../contracts/examples/nats/inventory-material-plan-netted-v1.json");
    const ORIGIN: Origin = Origin {
        intent: Intent {
            command_id: Uuid::from_u128(1),
            terms_revision: 1,
            policy_id: Uuid::from_u128(2),
            policy_revision: 1,
        },
        actor_id: Uuid::from_u128(3),
        issued_at: DateTime::from_millis(1_774_000_000_000),
    };

    fn fixture(split: bool) -> (Draft, Saved, [Supplier; 2]) {
        let draft = Draft::new(&read(&serde_json::from_str(FIXTURE).unwrap()).unwrap());
        let suppliers = [9, 8].map(|n| Supplier {
            id: Uuid::from_u128(n),
            code: format!("SUP-{n}"),
            name: "Historic supplier name".into(),
        });
        let prices = ["30.10", "149.95", "12.00", "7.35", "45.20"];
        let lines: Vec<_> = draft
            .plan
            .buy_rows
            .iter()
            .zip(prices)
            .enumerate()
            .map(|(index, (row, price))| {
                let (sku, revision) = row.key();
                json!({
                    "sku": sku, "revision": revision, "unit_price": price,
                    "supplier_id": suppliers[usize::from(split && index % 2 == 1)].id,
                })
            })
            .collect();
        let verified = suppliers
            .iter()
            .cloned()
            .map(|s| (s.id, s))
            .collect::<HashMap<_, _>>();
        let terms = command(&json!({
            "command_id": Uuid::from_u128(4), "expected_revision": 0, "lines": lines,
        }))
        .unwrap()
        .bind(&draft)
        .unwrap()
        .snapshot(1, &verified, Uuid::from_u128(5))
        .unwrap();
        (draft, terms, suppliers)
    }

    fn ids() -> impl Iterator<Item = Uuid> {
        (100..).map(Uuid::from_u128)
    }

    #[test]
    fn one_supplier_retains_five_exact_eur_lines_and_source_references() {
        let (draft, saved, suppliers) = fixture(false);
        let mut ids = ids();
        let batch = Batch::form(&draft, &saved, ORIGIN, APPROVED, || ids.next().unwrap()).unwrap();
        assert_eq!(batch.origin(), ORIGIN);
        assert_eq!(batch.batch_id(), Uuid::from_u128(100));
        assert_eq!(batch.plan_id(), draft.plan.plan_id);
        assert_eq!(batch.proposal_id, draft.proposal_id);
        assert_eq!(batch.source_order_id, draft.plan.order_id);
        assert_eq!(batch.source_line_id, draft.plan.line_id);
        assert_eq!(batch.currency, Currency::Eur);
        assert_eq!(batch.total.to_string(), "1882.40");
        let [order] = batch.orders.as_slice() else {
            panic!("expected one purchase order");
        };
        assert_eq!(order.order_id, Uuid::from_u128(101));
        assert_eq!(order.supplier, suppliers[0]);
        assert_eq!(order.total, batch.total);
        let actual = order.lines.iter().map(|line| {
            (
                line.row.key().0,
                line.row.whole_pieces().unwrap(),
                line.unit_price.to_string(),
                line.line_total.to_string(),
            )
        });
        let expected = [
            ("NM-ENDPLATE", 12, "30.10", "361.20"),
            ("NM-MOTOR", 6, "149.95", "899.70"),
            ("NM-RAIL", 8, "12.00", "96.00"),
            ("NM-SENSOR", 10, "7.35", "73.50"),
            ("NM-SPINDLE", 10, "45.20", "452.00"),
        ];
        assert!(actual.eq(
            expected.map(|(sku, pieces, unit, line)| {
                (sku, pieces, unit.to_owned(), line.to_owned())
            })
        ));
        assert!(
            order
                .lines
                .iter()
                .map(|line| line.line_id)
                .eq((102..107).map(Uuid::from_u128))
        );
        assert_eq!(ids.next(), Some(Uuid::from_u128(107)));
    }

    #[test]
    fn split_groups_use_supplier_id_order_and_reconcile_the_whole_approved_total() {
        let (mut draft, saved, suppliers) = fixture(true);
        let mut shuffled = serde_json::to_value(&saved).unwrap();
        shuffled["terms"]["lines"].as_array_mut().unwrap().reverse();
        let shuffled = serde_json::from_value(shuffled).unwrap();
        let form = |draft: &Draft, terms: &Saved| {
            let mut ids = ids();
            Batch::form(draft, terms, ORIGIN, APPROVED, || ids.next().unwrap()).unwrap()
        };
        let canonical = form(&draft, &saved);
        draft.plan.buy_rows.reverse();
        let batch = form(&draft, &shuffled);
        assert_eq!(batch, canonical);
        assert_eq!(batch.total.to_string(), "1882.40");
        let [first, second] = batch.orders.as_slice() else {
            panic!("expected two purchase orders");
        };
        for (order, supplier, total, rows) in [
            (
                first,
                &suppliers[1],
                "973.20",
                &["NM-MOTOR", "NM-SENSOR"][..],
            ),
            (
                second,
                &suppliers[0],
                "909.20",
                &["NM-ENDPLATE", "NM-RAIL", "NM-SPINDLE"][..],
            ),
        ] {
            assert_eq!(&order.supplier, supplier);
            assert_eq!(order.total.to_string(), total);
            assert!(
                order
                    .lines
                    .iter()
                    .map(|line| line.row.key().0)
                    .eq(rows.iter().copied())
            );
        }
        assert_eq!(first.total.checked_add(second.total), Ok(batch.total));
        assert_eq!(
            Batch::form(&draft, &saved, ORIGIN, first.total.as_u64(), || panic!(
                "allocated ID"
            )),
            Err(Failure::InconsistentTotal)
        );
    }

    #[test]
    fn inconsistent_snapshots_fail_before_allocating_ids() {
        let (draft, saved, _) = fixture(false);
        let original = serde_json::to_value(saved).unwrap();
        let check = |snapshot: Value, expected| {
            let terms = serde_json::from_value(snapshot).unwrap();
            assert_eq!(
                Batch::form(&draft, &terms, ORIGIN, APPROVED, || panic!("allocated ID")),
                Err(expected)
            );
        };

        let mut duplicate = original.clone();
        duplicate["terms"]["lines"][1]["sku"] = duplicate["terms"]["lines"][0]["sku"].clone();
        check(duplicate, Failure::DuplicateLineKey);
        for (field, value, failure) in [
            ("sku", json!(""), Failure::InvalidLineKey),
            ("revision", json!("?"), Failure::InvalidLineKey),
            ("pieces", json!("13"), Failure::InvalidSnapshot),
            ("pieces", json!("12.0"), Failure::InvalidSnapshot),
            ("line_total", json!("361.19"), Failure::InconsistentTotal),
            ("unit_price", json!("30.11"), Failure::InconsistentTotal),
        ] {
            let mut changed = original.clone();
            changed["terms"]["lines"][0][field] = value;
            check(changed, failure);
        }
        let mut supplier = original.clone();
        supplier["terms"]["lines"][0]["supplier"]["name"] = json!("A different historic name");
        check(supplier, Failure::InconsistentSupplier);
        let mut malformed = original.clone();
        malformed["terms"]["lines"][0]["supplier"]["code"] = json!("");
        check(malformed, Failure::InconsistentSupplier);
        let mut missing = original.clone();
        missing["terms"]["lines"].as_array_mut().unwrap().pop();
        check(missing, Failure::InvalidSnapshot);
        let mut wrong_proposal = original.clone();
        wrong_proposal["terms"]["proposal_id"] = json!(Uuid::new_v4());
        check(wrong_proposal, Failure::InvalidSnapshot);
        let mut wrong_revision = original.clone();
        wrong_revision["schemaVersion"] = json!(2);
        check(wrong_revision, Failure::InvalidSnapshot);
        let mut wrong_total = original.clone();
        wrong_total["terms"]["total"] = json!("1882.41");
        check(wrong_total, Failure::InconsistentTotal);
        // Agreement with the saved/approved total is not enough: all lines must add up to it.
        let mut misadded = original.clone();
        misadded["terms"]["lines"][0]["unit_price"] = json!("30.11");
        misadded["terms"]["lines"][0]["line_total"] = json!("361.32");
        check(misadded, Failure::InconsistentTotal);

        for (field, value) in [
            ("unit_price", json!("184467440737095516.16")),
            ("unit_price", json!("10000000.00")),
            ("unit", json!("KG")),
        ] {
            let mut invalid = original.clone();
            invalid["terms"]["lines"][0][field] = value;
            assert!(serde_json::from_value::<Saved>(invalid).is_err());
        }
    }

    #[test]
    fn rejects_invalid_quantities_even_if_the_stored_plan_row_also_contains_them() {
        let (mut draft, saved, _) = fixture(false);
        let original = serde_json::to_value(saved).unwrap();
        for pieces in ["0", "012", "12.5", "1000001", "18446744073709551616"] {
            let mut corrupted = original.clone();
            corrupted["terms"]["lines"][0]["pieces"] = json!(pieces);
            draft.plan.buy_rows[0] =
                serde_json::from_value(corrupted["terms"]["lines"][0].clone()).unwrap();
            assert_eq!(
                Batch::form(
                    &draft,
                    &serde_json::from_value(corrupted).unwrap(),
                    ORIGIN,
                    APPROVED,
                    || panic!("allocated ID"),
                ),
                Err(Failure::InvalidSnapshot)
            );
        }
    }

    #[test]
    fn rejects_stale_terms_and_nil_or_colliding_ids() {
        let (draft, saved, _) = fixture(false);
        let mut stale = ORIGIN;
        stale.intent.terms_revision = 2;
        assert_eq!(
            Batch::form(&draft, &saved, stale, APPROVED, || panic!("allocated ID")),
            Err(Failure::TermsMismatch)
        );
        let mut nil = ORIGIN;
        nil.intent.command_id = Uuid::nil();
        assert_eq!(
            Batch::form(&draft, &saved, nil, APPROVED, || panic!("allocated ID")),
            Err(Failure::InvalidIntent)
        );
        for bad in [Uuid::nil(), Uuid::from_u128(100)] {
            let mut ids = [Uuid::from_u128(100), bad].into_iter();
            assert_eq!(
                Batch::form(&draft, &saved, ORIGIN, APPROVED, || ids.next().unwrap()),
                Err(Failure::InvalidAllocatedId)
            );
        }
    }
}
