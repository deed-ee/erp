#!/bin/sh
set -eu

# Keep NUL-delimited Git paths intact and propagate Git and linter failures.
python3 - <<'PY'
import os
import subprocess
import sys

paths = subprocess.check_output([
    "git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "*.sh"
])
scripts = sorted({os.fsdecode(path) for path in paths.split(b"\0") if path and os.path.isfile(path)})
if scripts:
    sys.exit(subprocess.call(["shellcheck", "--", *scripts]))
PY
