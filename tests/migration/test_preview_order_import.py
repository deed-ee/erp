from copy import deepcopy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from jsonschema import Draft202012Validator, FormatChecker
from referencing import Registry, Resource


FIXTURES = Path(__file__).resolve().parent / "fixtures"
ROOT = FIXTURES.parents[2]
CONTRACTS = ROOT / "contracts/migration"


def validator(name):
    path = CONTRACTS / name
    registry = Registry().with_resources(
        (schema.as_uri(), Resource.from_contents(json.loads(schema.read_text())))
        for schema in CONTRACTS.glob("*.json")
    )
    Draft202012Validator.check_schema(json.loads(path.read_text()))
    return Draft202012Validator({"$ref": path.as_uri()}, registry=registry, format_checker=FormatChecker())


class PreviewTest(unittest.TestCase):
    def setUp(self):
        self.legacy = json.loads((FIXTURES / "legacy-import-orders.json").read_text())
        self.mapping = json.loads((FIXTURES / "article-mapping.json").read_text())
        self.context = ["--tenant-id", self.legacy["tenantId"], "--organization-id", self.legacy["organizationId"],
                        "--source-system", self.legacy["sourceSystem"], "--checkpoint", self.legacy["checkpoint"]]

    def run_preview(self, *options):
        with tempfile.TemporaryDirectory() as directory:
            files = {"legacy.json": json.dumps(self.legacy), "mapping.json": json.dumps(self.mapping)}
            for filename, content in files.items():
                (Path(directory) / filename).write_text(content)
            result = subprocess.run(
                [sys.executable, str(ROOT / "scripts/preview-order-import.py"), *self.context,
                 "--mapping-revision", "map-3", *options, *files],
                text=True, capture_output=True, check=False, timeout=10, cwd=directory,
            )
            self.assertEqual({p.name: p.read_text() for p in Path(directory).iterdir()}, files)
            return result

    def test_complete_preview_feeds_the_existing_reconciliation_tool(self):
        snapshot_validator = validator("order-snapshot-v1.json")
        snapshot_validator.validate(self.legacy)
        validator("article-mapping-v1.json").validate(self.mapping)
        result = self.run_preview()
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual((report["acceptedOrderCount"], report["rejectedOrderCount"]), (3, 0))
        self.assertEqual(report["mappingRevision"], "map-3")
        snapshot = report["normalizedSnapshot"]
        snapshot_validator.validate(snapshot)
        self.assertEqual(snapshot, json.loads((FIXTURES / "legacy-orders.json").read_text()))
        self.assertEqual(snapshot["orders"][1]["status"], "cancelled")

        with tempfile.TemporaryDirectory() as directory:
            legacy = Path(directory) / "normalized.json"
            legacy.write_text(json.dumps(snapshot))
            comparison = subprocess.run(
                [sys.executable, str(ROOT / "scripts/reconcile-orders.py"), *self.context,
                 str(legacy), str(FIXTURES / "candidate-orders.json")],
                capture_output=True, text=True, check=False, timeout=10,
            )
        self.assertEqual(comparison.returncode, 1, comparison.stderr)
        self.assertEqual(len(json.loads(comparison.stdout)["discrepancies"]), 4)

    def test_reports_every_problem_and_never_emits_partial_snapshot(self):
        self.mapping["mappings"].pop(1)
        self.mapping["mappings"][-1]["unit"] = "EA"
        result = self.run_preview()
        self.assertEqual(result.returncode, 1, result.stderr)
        report = json.loads(result.stdout)
        self.assertNotIn("normalizedSnapshot", report)
        self.assertEqual((report["acceptedOrderCount"], report["rejectedOrderCount"]), (1, 2))
        self.assertEqual(report["rejectedOrders"], [
            {"orderRef": "SO-100", "issues": [
                {"code": "unmappedArticle", "lineRef": "20", "sourceArticleRef": "OLD-SCREW"},
            ]},
            {"orderRef": "SO-300", "issues": [
                {"code": "unitMismatch", "lineRef": "10", "sourceArticleRef": "OLD-STEEL",
                 "actualUnit": "KG", "expectedUnit": "EA"},
            ]},
        ])
        self.legacy["orders"].reverse()
        self.legacy["orders"][-1]["lines"].reverse()
        self.mapping["mappings"].reverse()
        self.assertEqual(self.run_preview().stdout, result.stdout)

    def test_rejects_ambiguous_mapping_instead_of_using_the_last_row(self):
        mapping = self.mapping["mappings"][0]
        self.mapping["mappings"].append({**mapping, "targetArticleRef": "WRONG"})
        result = self.run_preview()
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")
        self.assertIn("duplicate article mapping reference", result.stderr)

    def test_no_report_for_untrusted_context_or_wrong_mapping_revision(self):
        original = deepcopy(self.mapping)
        for key, value in {"tenantId": "10000000-0000-4000-8000-000000000002",
                           "organizationId": "20000000-0000-4000-8000-000000000002",
                           "sourceSystem": "foreign-source", "mappingRevision": "stale-map"}.items():
            with self.subTest(field=key):
                self.mapping = {**original, key: value}
                result = self.run_preview()
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertNotIn(value, result.stderr)
                self.assertNotIn("SO-100", result.stderr)

    def test_rejects_malformed_input(self):
        self.legacy["orders"][0]["lines"][0]["remainingQuantity"] = 1.25
        result = self.run_preview()
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")
        self.assertIn("quantity", result.stderr)

    def test_change_preview_keeps_checkpoint_chain_and_cancellation(self):
        self.legacy = json.loads((FIXTURES / "legacy-import-change-0043.json").read_text())
        self.context[-1] = self.legacy["checkpoint"]
        window_validator = validator("order-change-window-v1.json")
        window_validator.validate(self.legacy)
        result = self.run_preview("--change-window")
        self.assertEqual(result.returncode, 0, result.stderr)
        window = json.loads(result.stdout)["normalizedWindow"]
        window_validator.validate(window)
        self.assertEqual(window, json.loads((FIXTURES / "order-change-0043.json").read_text()))
        self.assertEqual(window["orders"][1]["status"], "cancelled")
        self.mapping["mappings"].pop()
        blocked = self.run_preview("--change-window")
        self.assertEqual(blocked.returncode, 1, blocked.stderr)
        self.assertNotIn("normalizedWindow", json.loads(blocked.stdout))
