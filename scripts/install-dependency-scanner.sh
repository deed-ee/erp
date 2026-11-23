#!/bin/sh
# Install a reviewed Linux x86_64 release in a caller-supplied tools directory.
set -eu
case "$1" in
  trivy)
    url=https://github.com/aquasecurity/trivy/releases/download/v0.76.2/trivy_0.76.2_Linux-64bit.tar.gz
    sha=bc0fc726e017b8927a62b2d3531d7a84ee01769daf5c99ed866dced54089fdbf
    member=trivy ;;
  cargo-deny)
    url=https://github.com/EmbarkStudios/cargo-deny/releases/download/0.22.0/cargo-deny-0.22.0-x86_64-unknown-linux-musl.tar.gz
    sha=98c56e6a408bfa5c3c60744b6015aaa6aa08f743fdeb88386d06fca035bb1786
    member=cargo-deny-0.22.0-x86_64-unknown-linux-musl/cargo-deny ;;
  *) exit 2 ;;
esac
mkdir -p "$2"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
curl -fsSL --retry 3 -o "$tmp/tool.tar.gz" "$url"
printf '%s  %s\n' "$sha" "$tmp/tool.tar.gz" | sha256sum -c -
tar -xzf "$tmp/tool.tar.gz" -C "$tmp" "$member"
install -m 0755 "$tmp/$member" "$2/$1"
