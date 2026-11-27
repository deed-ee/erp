from copy import deepcopy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from jsonschema import Draft202012Validator, FormatChecker


FIXTURES = Path(__file__).resolve().parent / "fixtures"
ROOT = FIXTURES.parents[2]
TOOL = ROOT / "scripts/reconcile-orders.py"
SCHEMA = ROOT / "contracts/migration/order-snapshot-v1.json"


class ReconciliationTest(unittest.TestCase):
    def setUp(self):
        self.legacy = json.loads((FIXTURES / "legacy-orders.json").read_text())
        self.candidate = json.loads((FIXTURES / "candidate-orders.json").read_text())
        self.validator = Draft202012Validator(json.loads(SCHEMA.read_text()), format_checker=FormatChecker())

    def run_tool(self, candidate, *, expected=None):
        scope = self.legacy if expected is None else expected
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            legacy_file = directory / "legacy.json"
            candidate_file = directory / "candidate.json"
            legacy_text = json.dumps(self.legacy)
            candidate_text = candidate if isinstance(candidate, str) else json.dumps(candidate)
            legacy_file.write_text(legacy_text)
            candidate_file.write_text(candidate_text)
            result = subprocess.run(
                [sys.executable, str(TOOL), "--tenant-id", scope["tenantId"],
                 "--organization-id", scope["organizationId"], "--source-system", scope["sourceSystem"],
                 "--checkpoint", scope["checkpoint"], str(legacy_file), str(candidate_file)],
                capture_output=True, text=True, check=False, timeout=10, cwd=directory,
            )
            self.assertEqual({path.name for path in directory.iterdir()}, {"legacy.json", "candidate.json"})
            self.assertEqual(legacy_file.read_text(), legacy_text)
            self.assertEqual(candidate_file.read_text(), candidate_text)
            return result

    def test_examples_validate_against_contract(self):
        self.validator.check_schema(self.validator.schema)
        self.validator.validate(self.legacy)
        self.validator.validate(self.candidate)

    def test_discrepancies_despite_matching_counts_and_quantity_totals(self):
        result = self.run_tool(self.candidate)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(result.stderr, "")
        report = json.loads(result.stdout)
        self.assertEqual(report["checkpoint"], "order-export-0042")
        self.assertEqual(report["schemaVersion"], 1)
        self.assertFalse(report["matches"])
        self.assertEqual((report["legacyOrderCount"], report["candidateOrderCount"]), (3, 3))
        self.assertEqual(report["discrepancies"], [
            {"kind": "fieldMismatch", "orderRef": "SO-100", "lineRef": "10",
             "field": "remainingQuantity", "legacy": "6", "candidate": "4"},
            {"kind": "fieldMismatch", "orderRef": "SO-100", "lineRef": "20",
             "field": "remainingQuantity", "legacy": "4", "candidate": "6"},
            {"kind": "fieldMismatch", "orderRef": "SO-200", "field": "status",
             "legacy": "cancelled", "candidate": "open"},
            {"kind": "fieldMismatch", "orderRef": "SO-300", "field": "currency",
             "legacy": "EUR", "candidate": "CHF"},
        ])
        self.candidate["orders"].reverse()
        self.candidate["orders"][-1]["lines"].reverse()
        self.assertEqual(self.run_tool(self.candidate).stdout, result.stdout)

    def test_missing_unexpected_and_changed_references(self):
        self.candidate["orders"][1]["orderRef"] = "SO-250"
        self.candidate["orders"][0]["lines"][0]["lineRef"] = "30"
        line_a = self.candidate["orders"][0]["lines"][1]
        line_a.update(articleRef="WRONG", unit="BOX")
        self.candidate["orders"][0]["totalAmount"] = "100.01"
        result = self.run_tool(self.candidate)
        self.assertEqual(result.returncode, 1, result.stderr)
        discrepancies = json.loads(result.stdout)["discrepancies"]
        self.assertIn({"kind": "missingOrder", "orderRef": "SO-200"}, discrepancies)
        self.assertIn({"kind": "unexpectedOrder", "orderRef": "SO-250"}, discrepancies)
        self.assertIn({"kind": "missingLine", "orderRef": "SO-100", "lineRef": "20"}, discrepancies)
        self.assertIn({"kind": "unexpectedLine", "orderRef": "SO-100", "lineRef": "30"}, discrepancies)
        self.assertTrue({"unit", "articleRef", "totalAmount"} <= {diff.get("field") for diff in discrepancies})

    def test_matching_exact_decimals_and_reordered_rows(self):
        candidate = deepcopy(self.legacy)
        candidate["snapshotRole"] = "candidate"
        self.legacy["orders"][0]["totalAmount"] = "123456789123456789.123456789"
        candidate["orders"][0]["totalAmount"] = "123456789123456789.123456789"
        candidate["orders"][0]["lines"][0]["remainingQuantity"] = "6.0000"
        candidate["orders"][0]["lines"].reverse()
        candidate["orders"].reverse()
        result = self.run_tool(candidate)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(json.loads(result.stdout)["matches"])
        candidate["orders"][-1]["totalAmount"] = "123456789123456789.123456788"
        self.assertEqual(self.run_tool(candidate).returncode, 1)

    def test_mismatched_scope_source_and_checkpoint_never_emit_report(self):
        for field, value in {
            "tenantId": "10000000-0000-4000-8000-000000000002",
            "organizationId": "20000000-0000-4000-8000-000000000002",
            "sourceSystem": "another-source", "checkpoint": "wrong-checkpoint", "snapshotRole": "legacy",
        }.items():
            with self.subTest(field=field):
                result = self.run_tool({**self.candidate, field: value})
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertIn("does not match rehearsal", result.stderr)
                if field != "snapshotRole":
                    self.assertNotIn(value, result.stderr)
                self.assertNotIn("SO-100", result.stderr)

    def test_invalid_rows_and_format_are_not_business_differences(self):
        for field, value in (("totalAmount", 2.5), ("totalAmount", "2e3"),
                             ("totalAmount", "1.1234567890"), ("status", []),
                             ("currency", "eur"), ("orderRef", "abc\n"), ("lines", [])):
            with self.subTest(field=field, value=value):
                candidate = deepcopy(self.candidate)
                candidate["orders"][0][field] = value
                self.assertFalse(self.validator.is_valid(candidate))
                result = self.run_tool(candidate)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
        for version in (True, 2):
            result = self.run_tool({**self.candidate, "schemaVersion": version})
            self.assertEqual(result.returncode, 2)
            self.assertEqual(result.stdout, "")

    def test_duplicate_references_and_json_members_are_rejected(self):
        duplicated_order = deepcopy(self.candidate)
        duplicated_order["orders"].append(duplicated_order["orders"][0])
        duplicated_line = deepcopy(self.candidate)
        duplicated_line["orders"][0]["lines"].append(duplicated_line["orders"][0]["lines"][0])
        duplicate_member = json.dumps(self.candidate).replace('"schemaVersion": 1', '"schemaVersion": 1, "schemaVersion": 1')
        for invalid in (duplicated_order, duplicated_line, duplicate_member):
            result = self.run_tool(invalid)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(result.stdout, "")
            self.assertIn("duplicate", result.stderr)

    def test_unusable_inputs_fail_closed(self):
        for candidate in ("[" * 4000, '{"schemaVersion": NaN}', " " * (4 * 1024 * 1024 + 1)):
            result = self.run_tool(candidate)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(result.stdout, "")

    def test_empty_rehearsal_is_not_evidence_of_a_match(self):
        self.legacy["orders"] = []
        result = self.run_tool({**self.candidate, "orders": []})
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stdout, "")
        self.assertIn("no sample", result.stderr)


if __name__ == "__main__":
    unittest.main()
