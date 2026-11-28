"""Show that the pinned native tools accept good inputs and reject regressions."""

import os
from pathlib import Path
import subprocess
from tempfile import TemporaryDirectory
import tomllib

from openapi_spec_validator import validate
from openapi_spec_validator.validation.exceptions import OpenAPIValidationError


ROOT = Path(__file__).resolve().parents[2]
UI = ROOT / "ui"


def prove(label, command, directory, *, source=None, error=None):
    result = subprocess.run(command, cwd=directory, input=source, text=True, capture_output=True)
    output = result.stdout + result.stderr
    if error is None:
        assert result.returncode == 0, (label, output)
    else:
        assert result.returncode > 0 and error in output, (label, result.returncode, output)
    print(f"{label}: {'passed' if error is None else 'rejected as expected'}")


def prove_rust(sandbox):
    channel = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    cargo = ["cargo", f"+{channel}"]
    (sandbox / "Cargo.toml").write_text(
        '[package]\nname = "quality-fixture"\nversion = "0.0.0"\nedition = "2024"\n'
    )
    source = sandbox / "src/lib.rs"
    source.parent.mkdir()
    source.write_text("pub fn answer() -> bool {\n    true\n}\n")
    subprocess.run([*cargo, "generate-lockfile", "--offline"], cwd=sandbox, check=True)
    fmt = [*cargo, "fmt", "--all", "--", "--check"]
    lint = [*cargo, "clippy", "--locked", "--", "-D", "warnings"]
    prove("Rust formatting", fmt, sandbox)
    prove("Rust Clippy", lint, sandbox)
    source.write_text("pub fn answer()->bool{true}\n")
    prove("Rust formatting regression", fmt, sandbox, error="Diff in")
    source.write_text("pub fn answer(input: bool) -> bool {\n    if input { true } else { false }\n}\n")
    prove("Rust Clippy regression", lint, sandbox, error="clippy::needless_bool")


def prove_frontend():
    prettier = ["node_modules/.bin/prettier", "--check", "--stdin-filepath", "tests/quality-fixture.ts"]
    prove("UI formatting", prettier, UI, source="const count = 2;\n")
    prove("UI formatting regression", prettier, UI, source="const count=2\n", error="(stdin)")
    eslint = ["node_modules/.bin/eslint", "--stdin", "--stdin-filename", "tests/quality-fixture.ts"]
    prove("UI lint", eslint, UI, source="const count: number = 2;\n")
    prove("UI lint regression", eslint, UI, source="const count: any = 2;\n",
          error="@typescript-eslint/no-explicit-any")


def prove_contract():
    spec = {"openapi": "3.1.0", "info": {"title": "Quality fixture", "version": "1.0"}, "paths": {}}
    validate(spec)
    print("OpenAPI shape: passed")
    del spec["info"]["title"]
    try:
        validate(spec)
    except OpenAPIValidationError as exc:
        assert "'title' is a required property" in str(exc), str(exc)
        print("OpenAPI regression: rejected as expected")
    else:
        raise AssertionError("OpenAPI validator accepted a missing required title")


def prove_workflow():
    cmd = ["actionlint", "-shellcheck", "", "-pyflakes", "", "-"]
    sample = "on: push\njobs:\n  verify:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: 'true'\n"
    prove("Workflow shape", cmd, ROOT, source=sample)
    prove("Workflow regression", cmd, ROOT, source=sample.replace("    runs-on: ubuntu-24.04\n", ""),
          error='"runs-on" section is missing')


if __name__ == "__main__":
    with TemporaryDirectory() as folder:
        # Do not let a shared CARGO_TARGET_DIR contaminate other active worktrees.
        os.environ["CARGO_TARGET_DIR"] = str(Path(folder) / "target")
        prove_rust(Path(folder))
    prove_frontend()
    prove_contract()
    prove_workflow()
