#!/bin/sh
# Scan the installed bytes, not a registry alias. Shared by both CI checks.
set -eu
if [ "$#" -ne 2 ]; then
    printf '%s\n' 'Usage: scan-preview-identity.sh IMAGE OUTPUT_DIRECTORY' >&2
    exit 2
fi
repo=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
image=$(docker image inspect --format '{{.Id}}' "$1")
out=$2
mkdir -p "$out/tmp"
export TMPDIR="$out/tmp"
sh "$repo/scripts/install-dependency-scanner.sh" trivy "$out/tools"
scanner="$out/tools/trivy"
"$scanner" image --cache-dir "$out/cache" --image-src docker \
    --scanners vuln --severity HIGH,CRITICAL --format json \
    --output "$out/preview-identity.json" "$image"
"$scanner" convert --format table "$out/preview-identity.json"
jq -e '[.Results[].Vulnerabilities[]? | select((.FixedVersion // "") != "")] | length == 0' \
    "$out/preview-identity.json"
