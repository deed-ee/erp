"""Isolated positive and regression fixtures for the engineering quality checks."""

from hashlib import sha256
import json
from pathlib import Path
from runpy import run_path
import subprocess
from tempfile import TemporaryDirectory
import unittest


ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts/quality"
guidance = run_path(str(SCRIPTS / "check-guidance.py"))
workspace = run_path(str(SCRIPTS / "check-workspace.py"))
formatting = run_path(str(SCRIPTS / "check-ui-format.py"))
ci = run_path(str(SCRIPTS / "check-ci-results.py"))


class GuidanceTests(unittest.TestCase):
    def test_portable_metadata_and_invalid_variants(self):
        path = Path(".agents/skills/erp4us-demo/SKILL.md")
        valid = "---\nname: erp4us-demo\ndescription: A workflow for this test.\n---\nRead the checklist."
        self.assertEqual(guidance["check_skill"](path, valid)[0], [])
        for broken, message in (
            (valid.replace("erp4us-demo", "erp4us--demo"), "match its directory"),
            (valid.replace("A workflow for this test.", "[an array"), "expected"),
            (valid.replace("A workflow for this test.", "[]"), "description"),
            (valid.replace("\n---\n", "\nname: erp4us-demo\n---\n"), "unique strings"),
            (valid.replace("\n---\n", "\nallowed-tools: Bash\n---\n"), "unsupported frontmatter"),
        ):
            with self.subTest(message=message):
                issues, _ = guidance["check_skill"](path, broken)
                self.assertIn(message, "\n".join(issues))

    def test_existing_missing_and_outside_reference(self):
        with TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            doc = root / "README.md"
            doc.touch()
            valid = "[valid](README.md#heading)"
            self.assertEqual(guidance["check_links"](root, doc, valid), ([], {doc}))
            messages, _ = guidance["check_links"](
                root, doc, "[missing](docs/engineering/no-such-standard.md) [outside](../)"
            )
            self.assertEqual(len(messages), 2)
            self.assertIn("docs/engineering/no-such-standard.md", messages[0])


class WorkspaceTests(unittest.TestCase):
    def setUp(self):
        folder = TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        self.root = Path(folder.name).resolve()
        (self.root / "Cargo.toml").write_text("[workspace.lints.rust]\nunsafe_code = 'deny'\n")
        self.manifests = [
            Path("services/writer/Cargo.toml"), Path("services/reader/Cargo.toml"),
            Path("crates/common/Cargo.toml"),
        ]
        for path in self.manifests:
            (self.root / path).parent.mkdir(parents=True)
            (self.root / path).write_text("[lints]\nworkspace = true\n")
        self.metadata = {
            "workspace_root": str(self.root),
            "workspace_members": ["writer", "reader", "common"],
            "packages": [
                {"id": slug, "manifest_path": str(self.root / path), "dependencies": []}
                for slug, path in zip(("writer", "reader", "common"), self.manifests)
            ],
        }

    def test_missing_lints_and_orphan_are_rejected(self):
        self.assertEqual(workspace["check"](self.metadata, self.manifests), [])
        (self.root / self.manifests[0]).write_text("[lints.rust]\nunsafe_code = 'allow'\n")
        errors = workspace["check"](self.metadata, [*self.manifests, Path("services/orphan/Cargo.toml")])
        self.assertIn("add [lints] with workspace = true", errors[0])
        self.assertIn("include this first-party crate", errors[1])

    def test_services_and_shared_crates_cannot_import_foreign_owners(self):
        dependency = {"name": "reader", "rename": "renamed", "path": str(self.root / self.manifests[1].parent)}
        self.metadata["packages"][0]["dependencies"].append(dependency)
        self.metadata["packages"][2]["dependencies"].append(dependency)
        self.metadata["packages"][0]["dependencies"].append({"name": "async-nats", "path": None})
        errors = workspace["check"](self.metadata, self.manifests)
        self.assertEqual(len(errors), 3)
        self.assertEqual(sum("service-owned package reader" in issue for issue in errors), 2)
        self.assertTrue(any("direct async-nats" in issue for issue in errors))


class RatchetTests(unittest.TestCase):
    def test_changing_legacy_bytes_reenables_formatting(self):
        with TemporaryDirectory() as directory:
            path = Path(directory) / "sample.ts"
            path.write_bytes(b"let n=1\n")
            baseline = {path.as_posix(): sha256(path.read_bytes()).hexdigest()}
            self.assertFalse(formatting["needs_format_check"](path, baseline))
            path.write_bytes(b"let n=2\n")
            self.assertTrue(formatting["needs_format_check"](path, baseline))
            self.assertTrue(formatting["needs_format_check"](path, {}))

    def test_baseline_cannot_add_or_rehash_a_file(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "ui").mkdir()
            original = {"some.ts": "original", "other.ts": "other"}
            (root / "ui/format-baseline.json").write_text(json.dumps(original))
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(["git", "add", "ui"], cwd=root, check=True)
            tree = subprocess.check_output(["git", "write-tree"], cwd=root, text=True).strip()
            for changed, code in (({"some.ts": "original"}, 0), ({"some.ts": "new"}, 1), ({"new.ts": "new"}, 1)):
                result = subprocess.run(
                    ["python3", "-c",
                     "import json,sys,runpy; sys.exit(runpy.run_path(sys.argv[1])['verify_ratchet'](sys.argv[2], json.loads(sys.argv[3])))",
                     str(SCRIPTS / "check-ui-format.py"), tree, json.dumps(changed)],
                    cwd=root / "ui", capture_output=True,
                )
                self.assertEqual(result.returncode, code, result.stderr.decode())


class CiResultTests(unittest.TestCase):
    def test_allowed_skips_and_blocking_outcomes(self):
        result = {name: {"result": "success"} for name in ("impact", "rust", "infra", "compose", "audit", "contracts", "coverage")}
        result["impact"]["outputs"] = {"rust": "false", "audit": "false", "compose": "true"}
        for group in ("rust", "audit", "coverage"):
            result[group]["result"] = "skipped"
        self.assertEqual(ci["check"]("pull_request", result), [])
        for state in ("cancelled", "failure", "skipped"):
            result["compose"]["result"] = state
            self.assertIn(f"compose: expected success, got {state}", ci["check"]("pull_request", result))
        result["impact"]["result"] = "failure"
        self.assertIn("impact: expected success, got failure", ci["check"]("pull_request", result))

    def test_main_runs_full_checks_and_schedule_needs_coverage(self):
        result = {name: {"result": "success"} for name in ("impact", "rust", "infra", "compose", "audit", "contracts", "coverage")}
        result["impact"]["result"] = result["coverage"]["result"] = "skipped"
        self.assertEqual(ci["check"]("push", result), [])
        self.assertIn("coverage: expected success, got skipped", ci["check"]("schedule", result))
        result["coverage"]["result"] = "success"
        self.assertEqual(ci["check"]("schedule", result), [])
