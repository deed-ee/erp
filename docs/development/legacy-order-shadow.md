# Rehearse legacy order catch-up in a local sandbox

This is the durable catch-up rehearsal for [#108](https://github.com/erp4us-org/erp4us/issues/108). It uses a private, disposable SQLite file to represent a synthetic shadow copy. It does not write to Sales, Inventory, or any ERP API/database, run fulfillment, or change which system is authoritative.

## Apply an initial snapshot and changes

From the repository root, choose a new sandbox path in a private working directory. The sample initial snapshot is the normalized output of the [import preview](legacy-order-import-preview.md). A later [change window](../../tests/migration/fixtures/order-change-0043.json) reduces an outstanding quantity and cancels another order. For a simple shell rehearsal:

```sh
work=$(mktemp -d)
shadow() {
  checkpoint=$1
  shift
  python3 scripts/shadow-orders.py \
    --tenant-id 10000000-0000-4000-8000-000000000001 \
    --organization-id 20000000-0000-4000-8000-000000000001 \
    --source-system demo-trading-legacy \
    --checkpoint "$checkpoint" --sandbox "$work/shadow.sqlite" "$@"
}
shadow order-export-0042 initial tests/migration/fixtures/legacy-orders.json
shadow order-export-0043 apply tests/migration/fixtures/order-change-0043.json
shadow order-export-0043 apply tests/migration/fixtures/order-change-0043.json
shadow order-export-0043 export > "$work/candidate.json"
```

The first apply reports `applied`; the replay reports `alreadyApplied`. The export is a `candidate` under the [snapshot contract](../../contracts/migration/order-snapshot-v1.json) and contains all three orders, including the cancellation. Compare it with an **independent** source snapshot at `order-export-0043` using the [reconciliation tool](order-reconciliation.md). CI checks that comparison against a reference constructed independently of the sandbox.

## Apply rules and recovery

- Each [change window](../../contracts/migration/order-change-window-v1.json) contains complete replacement records for touched orders. Use the same normalized article keys as in the initial snapshot. Omitting an order leaves it unchanged. Represent cancellations explicitly; there are no delete operations. An empty window may advance the checkpoint without changing orders.
- A new window must reference the current checkpoint as `previousCheckpoint`. Order replacements and the recorded checkpoint commit in one transaction. If an apply fails, export using the old checkpoint and retry the original window after correcting the cause; the checkpoint cannot move ahead of applied data.
- Replaying a known window or initial file is a no-op, even after later windows have advanced the sandbox. Reusing a checkpoint with different records or predecessor is rejected. Ordering of JSON keys, orders and lines does not matter for replay; use the same original decimal strings.
- Each sandbox belongs to one tenant, organization and source-system namespace. Supply those identifiers and the expected checkpoint from the rehearsal plan. The tool verifies them locally; it does not authenticate an exporter or validate that an extract is complete. Use only trusted operators and synthetic inputs. Never use an existing application database path.
- Exit `0` means the command succeeded, including an exact replay. Exit `2` means no successful apply or export was emitted. A failed initial creation can leave an uninitialized file; choose a new sandbox path rather than treating it as a checkpoint. There is no production schema migration. To roll back the rehearsal, stop using the local sandbox; keep it if needed for evidence.

This proves the checkpoint, retry and comparison behavior on synthetic normalized inputs, not a supported legacy connector, production shadow deployment or authorization to cut over. Live exports and writes still need scoped service contracts; switching the leading system and fallback remain [#110](https://github.com/erp4us-org/erp4us/issues/110).
