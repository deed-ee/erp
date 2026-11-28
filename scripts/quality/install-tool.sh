#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    printf '%s\n' 'Usage: sh scripts/quality/install-tool.sh <actionlint|shellcheck|cargo-llvm-cov> <directory>' >&2
    exit 2
fi
if [ "$(uname -s)-$(uname -m)" != Linux-x86_64 ]; then
    printf '%s\n' 'Pinned download is for Linux x86_64; install the same versions for your platform.' >&2
    exit 2
fi
case "$1" in
    actionlint)
        url=https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz
        sha=8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8
        ;;
    shellcheck)
        url=https://github.com/koalaman/shellcheck/releases/download/v0.11.0/shellcheck-v0.11.0.linux.x86_64.tar.xz
        sha=8c3be12b05d5c177a04c29e3c78ce89ac86f1595681cab149b65b97c4e227198
        ;;
    cargo-llvm-cov)
        url=https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.9.2/cargo-llvm-cov-x86_64-unknown-linux-gnu.tar.gz
        sha=1e0ed5c64f11e9217f7b9d7c776b3125eab26f27d49313919c909881a2fc7cff
        ;;
    *) printf 'Unknown quality tool: %s\n' "$1" >&2; exit 2 ;;
esac

tmp=$(mktemp)
trap 'rm -f -- "$tmp"' EXIT
curl --fail --silent --show-error --location --retry 2 --max-time 90 "$url" -o "$tmp"
printf '%s  %s\n' "$sha" "$tmp" | sha256sum --check
mkdir -p -- "$2"
if [ "$1" = shellcheck ]; then
    tar -xJf "$tmp" -C "$2" --strip-components=1 shellcheck-v0.11.0/shellcheck
else
    tar -xzf "$tmp" -C "$2" "$1"
fi
