#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."

usage() {
    printf '%s\n' 'Usage: scripts/quality/check.sh [--all|--rust|--ui|--contracts|--guidance|--infra] ...'
}

run() {
    hint=$1
    shift
    printf '\n> %s\n' "$*"
    "$@" || {
        status=$?
        printf 'Check failed (%s). %s\nSee docs/engineering/quality-gates.md.\n' "$status" "$hint" >&2
        exit "$status"
    }
}

check_rust() {
    run 'Use the pinned Cargo toolchain; check missing lint inheritance or forbidden dependencies.' python3 scripts/quality/check-workspace.py
    run 'Run scripts/quality/format.sh --rust.' cargo fmt --all -- --check
    run 'Fix Clippy warnings rather than disabling rules.' cargo clippy --workspace --all-targets --locked -- -D warnings
    run 'Fix the failing behavior or test under the pinned toolchain.' cargo test --workspace --locked
}

check_ui() (
    cd ui
    run 'Install pinned dependencies and format the affected files with scripts/quality/format.sh --ui <paths>.' python3 ../scripts/quality/check-ui-format.py
    run 'Use Node from ui/.nvmrc and npm ci; address the reported rules.' npm run lint
    run 'Correct the Playwright types without disabling strict mode.' npm run typecheck:tests
    run 'Fix strict TypeScript and Angular template/build errors.' npm run build
)

check_contracts() {
    run 'Install tests/contracts/requirements.txt and fix the contract or fixture.' python3 tests/contracts/check.py
    run 'Validate the source records and regenerate the requirements index.' python3 tests/requirements/check.py
    run 'Inspect the failing catalog or impact fixture.' python3 -m unittest discover -s tests/requirements
}

check_guidance() {
    run 'Install tests/contracts/requirements.txt; fix the skill metadata or broken link.' python3 scripts/quality/check-guidance.py
    run 'Inspect the failing isolated engineering check fixture.' python3 -m unittest discover -s tests/quality
}

check_infra() {
    run 'Install pinned actionlint and ShellCheck; fix the reported workflow line.' actionlint -shellcheck shellcheck -pyflakes ''
    run 'Fix the reported shell file; ShellCheck must be on PATH.' sh scripts/quality/check-shell.sh
    run 'Install Docker Compose v2 and fix the base/overlay model.' docker compose \
        --env-file tests/quality/compose.env -f compose.yaml -f compose.delivery.yaml config --quiet
}

[ "$#" -ne 0 ] || set -- --all
for group in "$@"; do
    case "$group" in
        --all|--rust|--ui|--contracts|--guidance|--infra) ;;
        --help|-h) usage; exit 0 ;;
        *) usage >&2; exit 2 ;;
    esac
done

for group in "$@"; do
    case "$group" in
        --all) check_guidance; check_contracts; check_infra; check_rust; check_ui ;;
        --rust) check_rust ;;
        --ui) check_ui ;;
        --contracts) check_contracts ;;
        --guidance) check_guidance ;;
        --infra) check_infra ;;
    esac
done
