"""Select CI checks for a PR; unclassified inputs always run the full suite."""

import sys


EDITORIAL = {
    "AGENTS.md", "CHANGELOG.md", "CODE_OF_CONDUCT.md", "CONTRIBUTING.md",
    "GOVERNANCE.md", "README.md", "ROADMAP.md", "SECURITY.md",
}


def classify(paths):
    compose = False
    for path in paths:
        if path in EDITORIAL or (path.startswith("docs/") and path.endswith(".md")):
            continue
        if path.startswith("ui/"):
            compose = True
            continue
        # Integration-only fixtures can still be read by Rust tests. Treat any
        # other input as shared, including unknown services and local crates.
        return {"rust": True, "audit": True, "compose": True}
    return {"rust": False, "audit": False, "compose": compose}


def main():
    data = sys.stdin.buffer.read()
    if data and not data.endswith(b"\0"):
        raise ValueError("Expected NUL-terminated git diff paths")
    paths = data.decode(errors="surrogateescape").split("\0")[:-1]
    for name, needed in classify(paths).items():
        print(f"{name}={str(needed).lower()}")


if __name__ == "__main__":
    main()
