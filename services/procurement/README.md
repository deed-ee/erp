# Procurement: draft a proposal for a material-plan shortage

For [SC-05 / PUR-001](../../docs/requirements/catalog/PUR.yaml), Procurement uses Inventory's material plan to draft a recommendation for missing buy components. For example, if Inventory needs ten motors and can reserve six, the draft recommends buying four. A plan with no positive buy shortages produces no draft. The draft does not select a supplier, set prices, request approval, commit spend, or issue a purchase order.

## Runtime

`compose.procurement.yaml` extends the local database and delivery overlays. The one-shot initializer creates the `procurement_app` MongoDB user scoped to the `procurement` database. The API binds to loopback on `ERP4US_PROCUREMENT_PORT` (8094 by default). The runtime roles are:

- `serve`: [read a draft by plan ID](../../contracts/openapi/procurement-v1.yaml). The identity policy must grant `procurement.proposal.read` in the selected tenant and organization. Each lookup uses that verified scope; `404` means there is no draft there yet, or no shortages were emitted for the plan.
- `provision-consumer`: the operator command gets or creates only `procurement-material-plans-v1` on the existing `ERP4US_BUSINESS_EVENTS_V1` stream and refuses a mismatching configuration. It is run separately from the worker.
- `consume`: uses the restricted `procurement_inventory` broker login to attach to that exact tenant-filtered pull durable, with reply inbox `_INBOX.procurement-inventory`. It does not carry operator credentials or manage the stream.

The worker verifies the exact tenant subject, organization, producer, actor identity, correlation and causation, and the protocol and schema versions before writing a draft. These checks complement the broker's producer subject grants; the source header alone is not authentication. The worker saves draft and inbox receipt in one majority-acknowledged local transaction before acknowledging the delivery. `procurement.material_plan_drafts` uses a unique tenant/organization/plan key; `procurement.material_plan_inbox` uses a unique tenant/organization/message key. Identical broker replay and identical snapshots under new message IDs keep the original proposal ID and quantities, even if buy rows arrive in a different order. Rows are sorted by SKU, then revision (null first).

Invalid or out-of-scope events and conflicting snapshots go to `procurement.material_plan_quarantine`. Transient transaction failures get four delayed retries; after the final attempt only quarantine persistence is retried. The quarantine stores broker stream/sequence, reason, delivery count and creation metadata, not the message body. The worker terminates delivery only after that record is durable. Retain the data and durable while rolling back by stopping the API and worker; no legacy documents require migration.

## Legacy stdin adapter

For the original stock-shortage fixture (`procurement` with no arguments), run the demonstration from the repository root:

```sh
cargo build --locked -p procurement
python3 tests/integration/procurement.py
```

The test starts a disposable replica set and drives separate processes through their stdin/stdout contracts. The [shortage fixture](../../contracts/fixtures/stock-shortage-v1.json) stands in for a future Inventory event; it is not a broker envelope or a published event from Inventory. The adapter accepts [shortage snapshots](../../contracts/schemas/stock-shortage-v1.json) and [buyer lookups](../../contracts/schemas/get-draft-proposal-v1.json) by shortage ID. Successful JSON output is a draft object (`proposalId`, `status: "draft"`, `shortageId`, `orderId`, `lineId`, `articleId`, `unit`, `quantity`) or `null` when no draft is found or no units are missing. A zero shortage neither creates nor withdraws a draft.

Use a dedicated MongoDB account for the `procurement` database. Set `PROCUREMENT_MONGO_URI`, `PROCUREMENT_TENANT_ID`, and `PROCUREMENT_ORGANIZATION_ID` in the trusted process environment. Scope is not read from the payload. This fixture adapter has no network listener; it is for trusted local operators, not direct buyer or broker access. Build the runnable image with `docker build -f services/procurement/Dockerfile .`.

`procurement.drafts` stores the initial snapshot and creation metadata. The unique scoped shortage key and draft are inserted atomically in one MongoDB document. Retrying the same snapshot returns the existing proposal ID and quantity; a positive snapshot with the same scoped reference and different details is rejected. Revisions, withdrawal, supplier selection and purchasing approval remain out of scope. Roll back by stopping the adapter and retaining the database; no existing service collections are migrated.
