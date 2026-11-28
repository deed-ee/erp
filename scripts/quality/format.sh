#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."

case "${1:---rust}" in
    --rust)
        if [ "$#" -gt 1 ]; then exit 2; fi
        cargo fmt --all
        ;;
    --ui)
        shift
        if [ "$#" -eq 0 ]; then
            printf '%s\n' 'Select files relative to ui/, e.g. scripts/quality/format.sh --ui src/workspace.ts' >&2
            exit 2
        fi
        cd ui
        node_modules/.bin/prettier --write -- "$@"
        ;;
    *) printf '%s\n' 'Usage: scripts/quality/format.sh [--rust|--ui <paths relative to ui/>]' >&2; exit 2 ;;
esac
