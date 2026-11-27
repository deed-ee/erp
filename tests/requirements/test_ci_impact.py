from pathlib import Path
import runpy
import unittest


classify = runpy.run_path(str(Path(__file__).resolve().parents[2] / "scripts/ci-impact.py"))["classify"]


class ImpactTests(unittest.TestCase):
    def test_known_editorial_files_only_require_contract_checks(self):
        self.assertEqual(classify(["README.md", "docs/adr/0001-independent-services.md"]),
                         {"rust": False, "audit": False, "compose": False})

    def test_ui_still_requires_the_system_job(self):
        self.assertEqual(classify(["ui/package-lock.json", "docs/guide.md"]),
                         {"rust": False, "audit": False, "compose": True})

    def test_shared_mixed_and_unknown_changes_run_everything(self):
        for path in ("Cargo.lock", "crates/new/src/lib.rs", "services/new/build.rs",
                     "contracts/schemas/new.json", "compose.yaml", "tests/integration/new.py",
                     "docs/executable.sh", ".github/workflows/ci.yml", "new-root/file"):
            with self.subTest(path=path):
                self.assertTrue(all(classify(["ui/src/example.ts", path]).values()))
