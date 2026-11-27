#!/usr/bin/env python3
"""Preview a synthetic legacy order import without writing business records."""
import argparse
import json
from pathlib import Path
import sys

from migration_inputs import (
    REFERENCE, UNIT, InvalidInput, add_context_arguments, index_rows, matches, read_context,
    read_scoped, read_snapshot,
)


def read_mapping(path, expected):
    document = read_scoped(path, expected, {"mappings"}, "article mapping")

    def validate(row, number):
        if not matches(REFERENCE, row["targetArticleRef"]) or not matches(UNIT, row["unit"]):
            raise InvalidInput(f"invalid catalog reference or unit at mapping row {number}")
        return row

    return index_rows(document["mappings"], {"sourceArticleRef", "targetArticleRef", "unit"},
                      "sourceArticleRef", validate, "article mapping")


def preview(orders, mapping):
    normalized = []
    rejected = []
    for order_id, order in sorted(orders.items()):
        issues = []
        lines = []
        for line_id, line in sorted(order["lines"].items()):
            source = line["articleRef"]
            entry = mapping.get(source)
            context = {"lineRef": line_id, "sourceArticleRef": source}
            if entry is None:
                issues.append({"code": "unmappedArticle", **context})
            elif line["unit"] != entry["unit"]:
                issues.append({"code": "unitMismatch", **context,
                               "actualUnit": line["unit"], "expectedUnit": entry["unit"]})
            else:
                lines.append({**line, "articleRef": entry["targetArticleRef"]})
        if issues:
            rejected.append({"orderRef": order_id, "issues": issues})
        else:
            normalized.append({**order, "lines": lines})
    return normalized, rejected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    add_context_arguments(parser)
    parser.add_argument("--mapping-revision", required=True)
    parser.add_argument("legacy", type=Path)
    parser.add_argument("mapping", type=Path)
    args = parser.parse_args()
    expected = read_context(parser, args)
    if not matches(REFERENCE, args.mapping_revision):
        parser.error("invalid mapping revision")
    mapping_scope = {key: value for key, value in expected.items() if key != "checkpoint"}
    try:
        orders = read_snapshot(args.legacy, {**expected, "snapshotRole": "legacy"})
        mapping = read_mapping(args.mapping, {**mapping_scope, "mappingRevision": args.mapping_revision})
        if not orders:
            raise InvalidInput("no orders to preview")
    except InvalidInput as error:
        print(f"preview: {error}", file=sys.stderr)
        return 2
    normalized, rejected = preview(orders, mapping)
    report = {"schemaVersion": 1, **expected, "mappingRevision": args.mapping_revision,
              "acceptedOrderCount": len(normalized), "rejectedOrderCount": len(rejected),
              "rejectedOrders": rejected}
    # The output snapshot always covers the full source window, never a partial order set.
    if not rejected:
        report["normalizedSnapshot"] = {
            "schemaVersion": 1, "snapshotRole": "legacy", **expected, "orders": normalized,
        }
    print(json.dumps(report, indent=2))
    return 1 if rejected else 0


if __name__ == "__main__":
    sys.exit(main())
