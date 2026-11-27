# Preview a legacy order import

This is the offline validation step of [#108](https://github.com/erp4us-org/erp4us/issues/108). It takes a synthetic legacy order export and a versioned, organization-scoped article mapping supplied by the operator. It reads files and prints a report; it does not connect to an ERP or import records.

Run from the repository root with Python 3.12 or later:

```sh
python3 scripts/preview-order-import.py \
  --tenant-id 10000000-0000-4000-8000-000000000001 \
  --organization-id 20000000-0000-4000-8000-000000000001 \
  --source-system demo-trading-legacy \
  --checkpoint order-export-0042 \
  --mapping-revision map-3 \
  tests/migration/fixtures/legacy-import-orders.json \
  tests/migration/fixtures/article-mapping.json
```

The [order snapshot contract](../../contracts/migration/order-snapshot-v1.json) is the same as for [reconciliation](order-reconciliation.md), except that input article references are legacy codes. Each [mapping row](../../contracts/migration/article-mapping-v1.json) binds one source code to a target article key and exact unit. The preview never guesses a match or converts a unit. It retains external order and line references, status, amounts, quantities and checkpoint. If one line is blocked, the entire order is rejected. Correct the source or mapping and repeat at the same checkpoint.

| Exit | Meaning |
| --- | --- |
| `0` | Every order was accepted by the preview. Extract `normalizedSnapshot` from the report for use as the legacy input to reconciliation. This is not a write authorization. |
| `1` | At least one order has an unmapped article or unit mismatch. Find the source order and line in `rejectedOrders`. No partial normalized snapshot is emitted. |
| `2` | An input is unusable, e.g. invalid structure, duplicate mapping, another scope, unexpected checkpoint, or stale mapping revision. No report is emitted. |

For a [legacy change window](../../contracts/migration/order-change-window-v1.json), use `--change-window`, pass its new checkpoint and select the corresponding file. The passing report contains `normalizedWindow` instead of `normalizedSnapshot`, preserving both checkpoints. An empty change window is allowed; it explicitly records no touched orders.

Use independently reviewed synthetic inputs; the tool does not prove the mapping reflects a live target catalog. Treat reports with real references as confidential and do not commit them. Applying an accepted batch, retaining checkpoints across runs, and catching up after initial load remain open in #108. No database migration or routing change is required.

For a disposable demonstration of checkpointed apply and catch-up, see the [local shadow rehearsal](legacy-order-shadow.md). It uses normalized synthetic exports and does not import into a live service.
