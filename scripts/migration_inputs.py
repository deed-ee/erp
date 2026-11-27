"""Strict local inputs shared by the offline migration rehearsal commands."""
import json
import re


MAX_BYTES = 4 * 1024 * 1024
SCOPE_ID = re.compile(r"[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}")
NIL_ID = "00000000-0000-0000-0000-000000000000"
REFERENCE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}")
DECIMAL = re.compile(r"(?:0|[1-9][0-9]{0,17})(?:\.[0-9]{1,9})?")
UNIT = re.compile(r"[A-Z][A-Z0-9-]{0,15}")
CURRENCY = re.compile(r"[A-Z]{3}")
STATUSES = ("open", "cancelled", "fulfilled")


class InvalidInput(ValueError):
    """An offline migration input is not safe to process."""


def unique_members(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise InvalidInput("duplicate JSON member")
        result[key] = value
    return result


def reject_constant(_):
    raise InvalidInput("non-JSON numeric constant")


def matches(pattern, value):
    return isinstance(value, str) and pattern.fullmatch(value) is not None


def valid_scope_id(value):
    return matches(SCOPE_ID, value) and value != NIL_ID


def read_json(path):
    try:
        with path.open("rb") as stream:
            data = stream.read(MAX_BYTES + 1)
    except OSError:
        raise InvalidInput("cannot read input") from None
    if len(data) > MAX_BYTES:
        raise InvalidInput("input exceeds 4 MiB")
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=unique_members,
                              parse_constant=reject_constant)
    except InvalidInput:
        raise
    except (ValueError, RecursionError):
        raise InvalidInput("expected UTF-8 JSON") from None


def read_scoped(path, expected, fields, label):
    document = read_json(path)
    if not isinstance(document, dict) or document.keys() != {"schemaVersion", *expected, *fields}:
        raise InvalidInput(f"invalid {label} fields")
    if type(document["schemaVersion"]) is not int or document["schemaVersion"] != 1:
        raise InvalidInput(f"unsupported {label} version")
    # Never surface values from a document outside the operator-selected context.
    if any(document[key] != value for key, value in expected.items()):
        raise InvalidInput(f"{label} scope or input metadata does not match rehearsal")
    return document


def read_snapshot(path, expected):
    snapshot = read_scoped(path, expected, {"orders"}, "snapshot")
    return index_orders(snapshot["orders"])


def read_change_window(path, expected):
    window = read_scoped(path, expected, {"previousCheckpoint", "orders"}, "change window")
    previous = window["previousCheckpoint"]
    if not matches(REFERENCE, previous) or previous == expected["checkpoint"]:
        raise InvalidInput("invalid previous checkpoint")
    return previous, index_orders(window["orders"])


def index_rows(rows, shape, id_field, validate, label):
    if not isinstance(rows, list):
        raise InvalidInput(f"expected {label} array")
    indexed = {}
    for number, row in enumerate(rows, start=1):
        if not isinstance(row, dict) or row.keys() != shape:
            raise InvalidInput(f"invalid {label} fields at row {number}")
        identifier = row[id_field]
        if not matches(REFERENCE, identifier):
            raise InvalidInput(f"invalid {label} reference at row {number}")
        if identifier in indexed:
            raise InvalidInput(f"duplicate {label} reference at row {number}")
        indexed[identifier] = validate(row, number)
    return indexed


def index_orders(rows):
    def validate_order(order, number):
        if (order["status"] not in STATUSES or not matches(CURRENCY, order["currency"])
                or not matches(DECIMAL, order["totalAmount"])):
            raise InvalidInput(f"invalid status, amount or currency at order row {number}")

        def validate_line(line, line_number):
            if (not matches(REFERENCE, line["articleRef"]) or not matches(UNIT, line["unit"])
                    or not matches(DECIMAL, line["remainingQuantity"])):
                raise InvalidInput(f"invalid article, quantity or unit at order row {number}, line {line_number}")
            return line

        lines = index_rows(order["lines"], {"lineRef", "articleRef", "unit", "remainingQuantity"},
                           "lineRef", validate_line, f"line of order row {number}")
        if not lines:
            raise InvalidInput(f"order row {number} has no lines")
        return {**order, "lines": lines}

    return index_rows(rows, {"orderRef", "status", "totalAmount", "currency", "lines"},
                      "orderRef", validate_order, "order")


def add_context_arguments(parser):
    parser.add_argument("--tenant-id", required=True)
    parser.add_argument("--organization-id", required=True)
    parser.add_argument("--source-system", required=True)
    parser.add_argument("--checkpoint", required=True)


def read_context(parser, args):
    if not all(map(valid_scope_id, (args.tenant_id, args.organization_id))):
        parser.error("scope must use nonzero, canonical UUIDs")
    if not all(matches(REFERENCE, value) for value in (args.source_system, args.checkpoint)):
        parser.error("invalid source system or checkpoint")
    return {"tenantId": args.tenant_id, "organizationId": args.organization_id,
            "sourceSystem": args.source_system, "checkpoint": args.checkpoint}
