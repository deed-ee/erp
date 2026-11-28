"""Check new and changed UI sources without rewriting or hiding new formatting debt."""

from hashlib import sha256
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys


BASELINE = Path("format-baseline.json")


def needs_format_check(path, baseline):
    return baseline.get(path.as_posix()) != sha256(path.read_bytes()).hexdigest()


def verify_ratchet(base, current):
    key = f"ui/{BASELINE}"
    prior = subprocess.check_output(["git", "ls-tree", "--full-tree", base, "--", key])
    if not prior:
        return 0  # First adoption of the ratchet.
    original = json.loads(subprocess.check_output(["git", "show", f"{base}:{key}"]))
    additions = sorted(path for path, digest in current.items() if original.get(path) != digest)
    if additions:
        print("UI formatting baseline can only shrink; format: " + ", ".join(additions), file=sys.stderr)
        return 1
    return 0


def check_format(baseline):
    paths = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "."]
    )
    included = {".ts", ".mjs", ".cjs", ".html", ".css", ".json"}
    files = sorted({
        Path(name) for name in os.fsdecode(paths).split("\0")
        if Path(name).suffix in included and Path(name).is_file()
        and name not in {"package-lock.json", BASELINE.as_posix()}
        and not name.startswith("public/brand/")
    })
    matching = {path.as_posix() for path in files if not needs_format_check(path, baseline)}
    to_check = [f"./{path.as_posix()}" for path in files if path.as_posix() not in matching]
    print(f"Checking {len(to_check)} UI files; {len(files) - len(to_check)} match the legacy formatting baseline", flush=True)
    if to_check:
        result = subprocess.call(["node_modules/.bin/prettier", "--check", "--", *to_check])
        if result:
            return result
    stale = baseline.keys() - matching
    if stale:
        print(f"Remove stale format-baseline.json entries: {', '.join(sorted(stale))}", file=sys.stderr)
        return 1
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", help="verify that the tested merge tree only shrinks the base's ratchet")
    args = parser.parse_args()
    os.chdir(Path(__file__).resolve().parents[2] / "ui")
    baseline = json.loads(BASELINE.read_text(encoding="utf-8"))
    return verify_ratchet(args.base, baseline) if args.base else check_format(baseline)


if __name__ == "__main__":
    sys.exit(main())
