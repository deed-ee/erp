# Procurement: draft a proposal for a stock shortage

For [SC-05 / PUR-001](../../docs/requirements/catalog/PUR.yaml), when ten pieces are requested and six reserved, the buyer can retrieve one draft for the four missing pieces. The draft carries the original shortage, order, line, item and unit references. It does not select a supplier, commit spend, or send a purchase order.

Run the demonstration from the repository root:

```sh
cargo build --locked -p procurement
python3 tests/integration/procurement.py
```

The test starts a disposable replica set and drives separate processes through their stdin/stdout contracts. The [shortage fixture](../../contracts/fixtures/stock-shortage-v1.json) stands in for a future Inventory event; it is not a broker envelope or a published event from Inventory. The adapter accepts [shortage snapshots](../../contracts/schemas/stock-shortage-v1.json) and [buyer lookups](../../contracts/schemas/get-draft-proposal-v1.json) by shortage ID. Successful JSON output is a draft object (`proposalId`, `status: "draft"`, `shortageId`, `orderId`, `lineId`, `articleId`, `unit`, `quantity`) or `null` when no draft is found or no units are missing. A zero shortage neither creates nor withdraws a draft.

Use a dedicated MongoDB account for the `procurement` database. Set `PROCUREMENT_MONGO_URI`, `PROCUREMENT_TENANT_ID`, and `PROCUREMENT_ORGANIZATION_ID` in the trusted process environment. Scope is not read from the payload. This fixture adapter has no network listener; it is for trusted local operators, not direct buyer or broker access. A later network adapter must verify producer or buyer credentials and authorization before invoking Procurement. Build the runnable image with `docker build -f services/procurement/Dockerfile .`.

`procurement.drafts` stores the initial snapshot and creation metadata. The unique scoped shortage key and draft are inserted atomically in one MongoDB document. Retrying the same snapshot returns the existing proposal ID and quantity; a positive snapshot with the same scoped reference and different details is rejected. Revisions, withdrawal, supplier selection and purchasing approval remain out of scope. Roll back by stopping the adapter and retaining the database; no existing service collections are migrated.
