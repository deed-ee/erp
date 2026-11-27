#!/usr/bin/env python3
"""Compare two synthetic order snapshots; no service calls or data writes."""
import argparse
import json
from decimal import Decimal
from pathlib import Path
import re
import sys


MAX_BYTES = 4 * 1024 * 1024
SCOPE_ID = re.compile(r"[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}")
NIL_ID = "00000000-0000-0000-0000-000000000000"
REFERENCE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}")
DECIMAL = re.compile(r"(?:0|[1-9][0-9]{0,17})(?:\.[0-9]{1,9})?")
UNIT = re.compile(r"[A-Z][A-Z0-9-]{0,15}")
CURRENCY = re.compile(r"[A-Z]{3}")
STATUSES = ("open", "cancelled", "fulfilled")


class InvalidSnapshot(ValueError):
    """An input cannot be safely compared."""


def unique_members(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise InvalidSnapshot("duplicate JSON member")
        result[key] = value
    return result


def reject_constant(_):
    raise InvalidSnapshot("non-JSON numeric constant")


def matches(pattern, value):
    return isinstance(value, str) and pattern.fullmatch(value) is not None


def valid_scope_id(value):
    return matches(SCOPE_ID, value) and value != NIL_ID


def read_snapshot(path, expected):
    try:
        with path.open("rb") as stream:
            data = stream.read(MAX_BYTES + 1)
    except OSError:
        raise InvalidSnapshot("cannot read snapshot") from None
    if len(data) > MAX_BYTES:
        raise InvalidSnapshot("snapshot exceeds 4 MiB")
    try:
        snapshot = json.loads(data.decode("utf-8"), object_pairs_hook=unique_members,
                              parse_constant=reject_constant)
    except InvalidSnapshot:
        raise
    except (ValueError, RecursionError):
        raise InvalidSnapshot("expected UTF-8 JSON") from None
    if not isinstance(snapshot, dict) or snapshot.keys() != {"schemaVersion", "orders", *expected}:
        raise InvalidSnapshot("invalid snapshot fields")
    if type(snapshot["schemaVersion"]) is not int or snapshot["schemaVersion"] != 1:
        raise InvalidSnapshot("unsupported snapshot version")
    # Never surface values from a snapshot outside the operator-selected context.
    if any(snapshot[key] != value for key, value in expected.items()):
        raise InvalidSnapshot("scope, source system, checkpoint or snapshot role does not match rehearsal")
    return index_orders(snapshot["orders"])


def index_rows(rows, shape, id_field, validate, label):
    if not isinstance(rows, list):
        raise InvalidSnapshot(f"expected {label} array")
    indexed = {}
    for number, row in enumerate(rows, start=1):
        if not isinstance(row, dict) or row.keys() != shape:
            raise InvalidSnapshot(f"invalid {label} fields at row {number}")
        identifier = row[id_field]
        if not matches(REFERENCE, identifier):
            raise InvalidSnapshot(f"invalid {label} reference at row {number}")
        if identifier in indexed:
            raise InvalidSnapshot(f"duplicate {label} reference at row {number}")
        indexed[identifier] = validate(row, number)
    return indexed


def index_orders(rows):
    def validate_order(order, number):
        if (order["status"] not in STATUSES or not matches(CURRENCY, order["currency"])
                or not matches(DECIMAL, order["totalAmount"])):
            raise InvalidSnapshot(f"invalid status, amount or currency at order row {number}")

        def validate_line(line, line_number):
            if (not matches(REFERENCE, line["articleRef"]) or not matches(UNIT, line["unit"])
                    or not matches(DECIMAL, line["remainingQuantity"])):
                raise InvalidSnapshot(f"invalid article, quantity or unit at order row {number}, line {line_number}")
            return line

        lines = index_rows(order["lines"], {"lineRef", "articleRef", "unit", "remainingQuantity"},
                           "lineRef", validate_line, f"line of order row {number}")
        if not lines:
            raise InvalidSnapshot(f"order row {number} has no lines")
        return {**order, "lines": lines}

    return index_rows(rows, {"orderRef", "status", "totalAmount", "currency", "lines"},
                      "orderRef", validate_order, "order")


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
    parser.add_argument("--tenant-id", required=True)
    parser.add_argument("--organization-id", required=True)
    parser.add_argument("--source-system", required=True)
    parser.add_argument("--checkpoint", required=True)
    parser.add_argument("legacy", type=Path)
    parser.add_argument("candidate", type=Path)
    args = parser.parse_args()
    if not all(map(valid_scope_id, (args.tenant_id, args.organization_id))):
        parser.error("scope must use nonzero, canonical UUIDs")
    if not all(matches(REFERENCE, value) for value in (args.source_system, args.checkpoint)):
        parser.error("invalid source system or checkpoint")
    expected = {"tenantId": args.tenant_id, "organizationId": args.organization_id,
                "sourceSystem": args.source_system, "checkpoint": args.checkpoint}
    snapshots = {}
    for label in ("legacy", "candidate"):
        try:
            snapshots[label] = read_snapshot(getattr(args, label), {**expected, "snapshotRole": label})
        except InvalidSnapshot as error:
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
