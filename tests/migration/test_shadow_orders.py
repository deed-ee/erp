from concurrent.futures import ThreadPoolExecutor
from contextlib import closing
from copy import deepcopy
import json
from pathlib import Path
import sqlite3
import stat
import subprocess
import sys
import tempfile
import unittest

from test_preview_order_import import validator


FIXTURES = Path(__file__).resolve().parent / "fixtures"
ROOT = FIXTURES.parents[2]
SHADOW = ROOT / "scripts/shadow-orders.py"


class ShadowOrdersTest(unittest.TestCase):
    def setUp(self):
        self.work = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.database = self.work / "shadow.sqlite"
        self.initial_path = FIXTURES / "legacy-orders.json"
        self.change_path = FIXTURES / "order-change-0043.json"
        self.initial = json.loads(self.initial_path.read_text())
        self.window = json.loads(self.change_path.read_text())
        self.baseline = self.initial["checkpoint"]
        self.checkpoint = self.window["checkpoint"]

    def context(self, checkpoint, **overrides):
        scope = {**self.initial, **overrides}
        return ["--tenant-id", scope["tenantId"], "--organization-id", scope["organizationId"],
                "--source-system", scope["sourceSystem"], "--checkpoint", checkpoint]

    def command(self, checkpoint, command, *paths, **overrides):
        return subprocess.run(
            [sys.executable, str(SHADOW), *self.context(checkpoint, **overrides),
             "--sandbox", str(self.database), command, *map(str, paths)],
            capture_output=True, text=True, timeout=10, cwd=self.work, check=False,
        )

    def result(self, *args, **overrides):
        result = self.command(*args, **overrides)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def assert_rejected(self, *args, message, **overrides):
        result = self.command(*args, **overrides)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertIn(message, result.stderr)

    def put(self, data, name="window.json"):
        path = self.work / name
        path.write_text(json.dumps(data))
        return path

    def test_catch_up_then_reconcile_updated_quantities_and_cancellation(self):
        validator("order-change-window-v1.json").validate(self.window)
        self.assertEqual(self.result(self.baseline, "initial", self.initial_path)["result"], "applied")
        self.assertEqual(stat.S_IMODE(self.database.stat().st_mode), 0o600)
        preview = subprocess.run(
            [sys.executable, str(ROOT / "scripts/preview-order-import.py"), *self.context(self.checkpoint),
             "--mapping-revision", "map-3", "--change-window", str(FIXTURES / "legacy-import-change-0043.json"),
             str(FIXTURES / "article-mapping.json")],
            text=True, capture_output=True, timeout=10, check=False,
        )
        self.assertEqual(preview.returncode, 0, preview.stderr)
        applied = self.result(self.checkpoint, "apply", self.put(json.loads(preview.stdout)["normalizedWindow"]))
        self.assertEqual((applied["result"], applied["orderCount"], applied["currentCheckpoint"]),
                         ("applied", 2, self.checkpoint))
        snapshot = self.result(self.checkpoint, "export")
        validator("order-snapshot-v1.json").validate(snapshot)
        self.assertEqual([o["orderRef"] for o in snapshot["orders"]], ["SO-100", "SO-200", "SO-300"])
        self.assertEqual(snapshot["orders"][0]["lines"][0]["remainingQuantity"], "3")
        self.assertEqual(snapshot["orders"][1], self.initial["orders"][1])
        self.assertEqual(snapshot["orders"][2]["status"], "cancelled")
        self.assertEqual(snapshot["orders"][2]["lines"][0]["remainingQuantity"], "0")

        # An independently constructed legacy snapshot at the new source checkpoint.
        expected = {**self.initial, "checkpoint": self.checkpoint,
                    "orders": [self.window["orders"][0], self.initial["orders"][1], self.window["orders"][1]]}
        compare = subprocess.run(
            [sys.executable, str(ROOT / "scripts/reconcile-orders.py"), *self.context(self.checkpoint),
             str(self.put(expected, "legacy-final.json")), str(self.put(snapshot, "shadow-final.json"))],
            text=True, capture_output=True, timeout=10, check=False,
        )
        self.assertEqual(compare.returncode, 0, compare.stderr)
        self.assertTrue(json.loads(compare.stdout)["matches"])

    def test_exact_replay_does_not_rewind_current_checkpoint(self):
        self.result(self.baseline, "initial", self.initial_path)
        self.result(self.checkpoint, "apply", self.change_path)
        later = "order-export-0044"
        empty = self.put({**self.window, "previousCheckpoint": self.checkpoint, "checkpoint": later, "orders": []})
        self.result(later, "apply", empty)
        before = self.result(later, "export")
        reordered = deepcopy(self.window)
        reordered["orders"].reverse()
        reordered["orders"][-1]["lines"].reverse()
        replay = self.result(self.checkpoint, "apply", self.put(reordered))
        self.assertEqual((replay["result"], replay["currentCheckpoint"]), ("alreadyApplied", later))
        replay_initial = self.result(self.baseline, "initial", self.initial_path)
        self.assertEqual((replay_initial["result"], replay_initial["currentCheckpoint"]), ("alreadyApplied", later))
        self.assertEqual(self.result(later, "export"), before)

    def test_gaps_and_changed_reuses_do_not_change_state(self):
        self.result(self.baseline, "initial", self.initial_path)
        wrong_previous = self.put({**self.window, "previousCheckpoint": "missing"})
        self.assert_rejected(self.checkpoint, "apply", wrong_previous, message="does not continue")
        self.result(self.checkpoint, "apply", self.change_path)
        before = self.result(self.checkpoint, "export")
        altered = deepcopy(self.window)
        altered["orders"][0]["status"] = "fulfilled"
        self.assert_rejected(self.checkpoint, "apply", self.put(altered), message="different input")
        self.assert_rejected(self.baseline, "export", message="requested checkpoint")
        self.assertEqual(self.result(self.checkpoint, "export"), before)

    def test_mid_window_failure_rolls_back_orders_and_checkpoint_then_resumes(self):
        self.result(self.baseline, "initial", self.initial_path)
        before = self.result(self.baseline, "export")
        with closing(sqlite3.connect(self.database)) as connection, connection:
            connection.execute("""
                CREATE TRIGGER fail_second BEFORE UPDATE ON orders
                WHEN new.order_ref = 'SO-300'
                BEGIN SELECT RAISE(ABORT, 'test interruption'); END
            """)
        self.assert_rejected(self.checkpoint, "apply", self.change_path, message="unable to process local sandbox")
        self.assertEqual(self.result(self.baseline, "export"), before)
        self.assert_rejected(self.checkpoint, "export", message="requested checkpoint")
        with closing(sqlite3.connect(self.database)) as connection, connection:
            connection.execute("DROP TRIGGER fail_second")
        self.assertEqual(self.result(self.checkpoint, "apply", self.change_path)["result"], "applied")
        self.assertEqual(self.result(self.checkpoint, "export")["orders"][2]["status"], "cancelled")

    def test_concurrent_replay_is_applied_once(self):
        self.result(self.baseline, "initial", self.initial_path)
        with ThreadPoolExecutor(max_workers=2) as pool:
            results = list(pool.map(lambda _: self.command(self.checkpoint, "apply", self.change_path), range(2)))
        self.assertTrue(all(r.returncode == 0 for r in results), [r.stderr for r in results])
        self.assertEqual(sorted(json.loads(r.stdout)["result"] for r in results), ["alreadyApplied", "applied"])

    def test_scope_mismatch_does_not_reveal_orders_or_write(self):
        self.result(self.baseline, "initial", self.initial_path)
        before = self.database.read_bytes()
        for field, value in {"tenantId": "10000000-0000-4000-8000-000000000002",
                             "organizationId": "20000000-0000-4000-8000-000000000002",
                             "sourceSystem": "unrelated-erp"}.items():
            with self.subTest(field=field):
                self.assert_rejected(self.baseline, "export", message="sandbox scope", **{field: value})
                self.assert_rejected(self.checkpoint, "apply", self.put({**self.window, field: value}),
                                     message="sandbox scope", **{field: value})
        self.assertEqual(self.database.read_bytes(), before)

    def test_does_not_create_missing_database_or_overwrite_unrelated_file(self):
        self.assert_rejected(self.checkpoint, "apply", self.change_path, message="unable to process")
        self.assertFalse(self.database.exists())
        self.database.write_bytes(b"this is not a rehearsal database")
        self.assert_rejected(self.baseline, "initial", self.initial_path, message="unable to process")
        self.assertEqual(self.database.read_bytes(), b"this is not a rehearsal database")
