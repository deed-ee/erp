"""Require all CI checks selected for the tested revision to succeed."""

import json
import os
import sys


def check(event, needs):
    impact = needs["impact"]
    result = {"infra": "success", "contracts": "success"}
    result["impact"] = "success" if event == "pull_request" else "skipped"
    result["coverage"] = "success" if event in {"schedule", "workflow_dispatch"} else "skipped"
    for group in ("rust", "compose", "audit"):
        skip = (
            event == "pull_request" and impact["result"] == "success"
            and impact["outputs"].get(group) == "false"
        )
        result[group] = "skipped" if skip else "success"
    return [
        f"{name}: expected {expected}, got {actual}"
        for name, expected in result.items()
        if (actual := needs[name]["result"]) != expected
    ]


if __name__ == "__main__":
    errors = check(os.environ["GITHUB_EVENT_NAME"], json.loads(os.environ["NEEDS_JSON"]))
    if errors:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(1)
    print("All selected CI checks passed")
