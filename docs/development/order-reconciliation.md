# Rehearse parallel ERP reconciliation

This offline exercise for [#109](https://github.com/erp4us-org/erp4us/issues/109) compares synthetic order snapshots for one organization. It does not connect to either ERP, authenticate exporters, apply an import, or change order routing. The legacy system remains authoritative; the script only reads local inputs and writes a report to standard output. No shipments, invoices, purchase orders or notifications are sent by this tool.

## Run the sample

From the repository root with Python 3.12 or later:

```sh
python3 scripts/reconcile-orders.py \
  --tenant-id 10000000-0000-4000-8000-000000000001 \
  --organization-id 20000000-0000-4000-8000-000000000001 \
  --source-system demo-trading-legacy \
  --checkpoint order-export-0042 \
  tests/migration/fixtures/legacy-orders.json \
  tests/migration/fixtures/candidate-orders.json
```

The report finds four discrepancies even though both sides have three orders: swapped remaining quantities on two lines of `SO-100`, a missed cancellation on `SO-200` and a currency mismatch on `SO-300`. Reports are sorted by order and line reference so reordering input records does not reorder the results.

| Exit | Meaning |
| --- | --- |
| `0` | Both snapshots are comparable and no differences were found in the supplied records. |
| `1` | Comparable snapshots, with differences in the JSON report. Resolve or investigate these before using the rehearsal as evidence. |
| `2` | Unusable inputs (for example, wrong scope or checkpoint, duplicated references, or both snapshots empty). No report is emitted. |

## Prepare comparable snapshots

1. Produce independent, authorized extracts: `legacy` from the leading system and `candidate` from the shadow copy. Use the same order selection, one tenant and organization, the same source-system namespace and a common source checkpoint. Both extracts must include orders that were cancelled or fulfilled during the agreed window, not just orders that are still open.
2. Normalize the exports to the [v1 snapshot contract](../../contracts/migration/order-snapshot-v1.json). Preserve stable external order and line references and map the article references to the same catalog keys. Choose one definition of the amount for this rehearsal (gross order total in transaction currency in the sample); `remainingQuantity` is the unfulfilled quantity in the line's stated unit. The comparison never converts units or currencies and never applies a tolerance. Amounts and quantities are nonnegative exact decimal strings, not JSON floats.
3. Run the script with the expected identifiers from the rehearsal plan, not identifiers copied blindly from the input files. An invalid snapshot is rejected without emitting its record values. Limit files to 4 MiB each. Keep extracts and reports local; reports contain source references and commercial values. Do not commit live exports.
4. Inspect differences by order and line. Missing or unexpected orders/lines are separate from differences in values. Fix source mapping or import problems and repeat the rehearsal against a common checkpoint. `matches: true` describes the supplied records only; it is not proof of completeness or permission to change the leading system.

This slice does not execute the legacy import or catch-up in [#108](https://github.com/erp4us-org/erp4us/issues/108), prove a production shadow environment suppresses outbound actions, or perform the cutover and fallback in [#110](https://github.com/erp4us-org/erp4us/issues/110). The synthetic contract can be replaced by reviewed export adapters later. There is no database migration to revert.

If the legacy snapshot still uses source article codes, run the [offline import preview](legacy-order-import-preview.md) first. It emits a normalized snapshot only when every order maps cleanly.
