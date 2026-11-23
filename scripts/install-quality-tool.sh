#!/bin/sh
set -eu

case "$1" in
    actionlint)
        url=https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz
        sha=8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8
        ;;
    cargo-llvm-cov)
        url=https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.9.2/cargo-llvm-cov-x86_64-unknown-linux-gnu.tar.gz
        sha=1e0ed5c64f11e9217f7b9d7c776b3125eab26f27d49313919c909881a2fc7cff
        ;;
    *)
        printf 'Unknown quality tool: %s\n' "$1" >&2
        exit 2
        ;;
esac

tmp=$(mktemp)
trap 'rm -f -- "$tmp"' EXIT
curl --fail --silent --show-error --location --retry 2 --max-time 60 "$url" -o "$tmp"
printf '%s  %s\n' "$sha" "$tmp" | sha256sum --check --status
mkdir -p -- "$2"
tar -xzf "$tmp" -C "$2" "$1"
