"""Validate lint inheritance and dependencies using Cargo's own workspace metadata."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib


def read_manifest(path):
    with path.open("rb") as stream:
        return tomllib.load(stream)


def service_owner(relative_path):
    parts = relative_path.parts
    return parts[1] if len(parts) > 1 and parts[0] == "services" else None


def check(metadata, tracked_manifests):
    root = Path(metadata["workspace_root"])
    members = set(metadata["workspace_members"])
    workspace_manifests = set()
    errors = []
    if not read_manifest(root / "Cargo.toml").get("workspace", {}).get("lints"):
        errors.append("Cargo.toml: missing workspace lint policy")
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        manifest = Path(package["manifest_path"])
        relative = manifest.relative_to(root)
        workspace_manifests.add(relative)
        if read_manifest(manifest).get("lints", {}).get("workspace") is not True:
            errors.append(f"{relative}: add [lints] with workspace = true")
        for dependency in package["dependencies"]:
            path = dependency.get("path")
            target = Path(path).resolve() if path else None
            target_owner = service_owner(target.relative_to(root)) if target and target.is_relative_to(root) else None
            if target_owner is not None and target_owner != service_owner(relative):
                errors.append(f"{relative}: must not depend on service-owned package {dependency['name']}")
            if dependency["name"] == "async-nats" and relative != Path("crates/erp4us-messaging/Cargo.toml"):
                errors.append(f"{relative}: use erp4us-messaging instead of direct async-nats")
    for path in sorted(set(tracked_manifests) - workspace_manifests):
        errors.append(f"{path}: include this first-party crate in the workspace")
    return errors


def main():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"]
    ))
    paths = subprocess.check_output([
        "git", "ls-files", "-z", "--cached", "--others", "--exclude-standard",
        "--", "services", "crates",
    ])
    manifests = [
        Path(item) for item in os.fsdecode(paths).split("\0")
        if item.endswith("/Cargo.toml") and Path(item).is_file()
    ]
    errors = check(metadata, manifests)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Validated {len(metadata['workspace_members'])} Cargo workspace members")
    return 0


if __name__ == "__main__":
    sys.exit(main())
