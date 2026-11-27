from copy import deepcopy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


FIXTURES = Path(__file__).resolve().parent / "fixtures"
SCRIPTS = FIXTURES.parents[2] / "scripts"


class CutoverWalkthroughTest(unittest.TestCase):
    def setUp(self):
        self.work = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.final = json.loads((FIXTURES / "legacy-cutover-final.json").read_text())
        self.context = ["--tenant-id", self.final["tenantId"], "--organization-id", self.final["organizationId"],
                        "--source-system", self.final["sourceSystem"]]

    def invoke(self, tool, checkpoint, *args):
        return subprocess.run(
            [sys.executable, str(SCRIPTS / tool), *self.context, "--checkpoint", checkpoint, *map(str, args)],
            capture_output=True, text=True, check=False, timeout=10, cwd=self.work,
        )

    def snapshot(self, name, data):
        path = self.work / name
        path.write_text(json.dumps(data))
        return path

    def assert_matches(self, result, *, matches):
        self.assertEqual(result.returncode, 0 if matches else 1, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["matches"], matches)
        return report

    def test_cutover_data_check_blocks_stale_or_disagreeing_candidate(self):
        store = self.work / "shadow.sqlite"
        initial = self.invoke("shadow-orders.py", "order-export-0042",
                              "--sandbox", store, "initial", FIXTURES / "legacy-orders.json")
        self.assertEqual(initial.returncode, 0, initial.stderr)
        checkpoint = self.final["checkpoint"]
        stale = self.invoke("shadow-orders.py", checkpoint, "--sandbox", store, "export")
        self.assertEqual(stale.returncode, 2, stale.stderr)
        self.assertEqual(stale.stdout, "")
        applied = self.invoke("shadow-orders.py", checkpoint,
                              "--sandbox", store, "apply", FIXTURES / "order-change-0043.json")
        self.assertEqual(applied.returncode, 0, applied.stderr)
        exported = self.invoke("shadow-orders.py", checkpoint, "--sandbox", store, "export")
        self.assertEqual(exported.returncode, 0, exported.stderr)

        expected = FIXTURES / "legacy-cutover-final.json"
        candidate = json.loads(exported.stdout)
        bad = deepcopy(candidate)
        bad["orders"][-1]["status"] = "open"
        report = self.assert_matches(self.invoke("reconcile-orders.py", checkpoint, expected,
                                     self.snapshot("missed-cancellation.json", bad)), matches=False)
        self.assertEqual(report["discrepancies"], [
            {"kind": "fieldMismatch", "orderRef": "SO-300", "field": "status",
             "legacy": "cancelled", "candidate": "open"},
        ])
        self.assert_matches(self.invoke("reconcile-orders.py", checkpoint, expected,
                            self.snapshot("caught-up.json", candidate)), matches=True)

    def test_fallback_data_check_accounts_for_post_switch_acceptance(self):
        checkpoint = "handback-export-0001"
        expected = {**self.final, "checkpoint": checkpoint}
        observed = {**deepcopy(expected), "snapshotRole": "candidate"}
        before = self.snapshot("expected-before-new-writes.json", expected)
        observed_path = self.snapshot("legacy-recovery.json", observed)
        self.assert_matches(self.invoke("reconcile-orders.py", checkpoint, before, observed_path), matches=True)

        # Synthetic acceptance after the switch, not a real Sales command.
        accepted = {
            "orderRef": "SO-900", "status": "open", "currency": "EUR", "totalAmount": "9.00",
            "lines": [{"lineRef": "10", "articleRef": "BRACKET", "unit": "EA", "remainingQuantity": "1"}],
        }
        expected["orders"].append(accepted)
        after = self.snapshot("expected-after-new-writes.json", expected)
        report = self.assert_matches(self.invoke("reconcile-orders.py", checkpoint, after, observed_path), matches=False)
        self.assertEqual(report["discrepancies"], [{"kind": "missingOrder", "orderRef": "SO-900"}])
        observed["orders"].append(deepcopy(accepted))
        recovered = self.snapshot("legacy-recovered.json", observed)
        self.assert_matches(self.invoke("reconcile-orders.py", checkpoint, after, recovered), matches=True)

        observed["orders"].append(deepcopy(accepted))
        duplicate = self.invoke("reconcile-orders.py", checkpoint, after,
                                self.snapshot("duplicate-handback.json", observed))
        self.assertEqual(duplicate.returncode, 2, duplicate.stderr)
        self.assertEqual(duplicate.stdout, "")
        self.assertIn("duplicate order reference", duplicate.stderr)
