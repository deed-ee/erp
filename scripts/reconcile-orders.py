#!/usr/bin/env python3
"""Compare two synthetic order snapshots; no service calls or data writes."""
import argparse
import json
from decimal import Decimal
from pathlib import Path
import sys

from migration_inputs import InvalidInput, add_context_arguments, read_context, read_snapshot


def compare_fields(left, right, fields, context, discrepancies):
    for field in fields:
        before, after = left[field], right[field]
        if field in ("totalAmount", "remainingQuantity"):
            # Equality needs no rounding, arithmetic context or currency conversion.
            if Decimal(before) == Decimal(after):
                continue
        elif before == after:
            continue
        discrepancies.append({"kind": "fieldMismatch", **context, "field": field,
                              "legacy": before, "candidate": after})


def reconcile(legacy, candidate):
    discrepancies = []
    for order_id in sorted(legacy.keys() | candidate.keys()):
        order = {"orderRef": order_id}
        if order_id not in candidate:
            discrepancies.append({"kind": "missingOrder", **order})
            continue
        if order_id not in legacy:
            discrepancies.append({"kind": "unexpectedOrder", **order})
            continue
        before, after = legacy[order_id], candidate[order_id]
        compare_fields(before, after, ("status", "totalAmount", "currency"), order, discrepancies)
        old_lines, new_lines = before["lines"], after["lines"]
        for line_id in sorted(old_lines.keys() | new_lines.keys()):
            line = {**order, "lineRef": line_id}
            if line_id not in new_lines:
                discrepancies.append({"kind": "missingLine", **line})
            elif line_id not in old_lines:
                discrepancies.append({"kind": "unexpectedLine", **line})
            else:
                compare_fields(old_lines[line_id], new_lines[line_id],
                               ("articleRef", "remainingQuantity", "unit"), line, discrepancies)
    return discrepancies


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    add_context_arguments(parser)
    parser.add_argument("legacy", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()
    expected = read_context(parser, args)
    snapshots = {}
    for label in ("legacy", "candidate"):
        try:
            snapshots[label] = read_snapshot(getattr(args, label), {**expected, "snapshotRole": label})
        except InvalidInput as error:
            print(f"{label}: {error}", file=sys.stderr)
            return 2
    if not any(snapshots.values()):
        print("no orders in either snapshot; there is no sample to reconcile", file=sys.stderr)
        return 2
    discrepancies = reconcile(snapshots["legacy"], snapshots["candidate"])
    report = {"schemaVersion": 1, **expected, "matches": not discrepancies,
              "legacyOrderCount": len(snapshots["legacy"]),
              "candidateOrderCount": len(snapshots["candidate"]),
              "discrepancies": discrepancies}
    print(json.dumps(report, indent=2))
    return 1 if discrepancies else 0


if __name__ == "__main__":
    sys.exit(main())
