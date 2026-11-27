#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
python3 scripts/preview-identity.py prepare
export ERP4US_IDENTITY_DIR=./.erp4us-preview
export ERP4US_ENV_FILE="$PWD/.erp4us-preview/compose.env"
export COMPOSE_PROJECT_NAME=erp4us-preview
if [ ! -f "$ERP4US_ENV_FILE" ]; then sh scripts/init-dev.sh; fi
if ! grep -q '^DEV_NATS_DIAGNOSTICS_PASSWORD=' "$ERP4US_ENV_FILE"; then
    password=$(openssl rand -hex 32)
    printf 'DEV_NATS_DIAGNOSTICS_PASSWORD=%s\n' "$password" >> "$ERP4US_ENV_FILE"
    unset password
fi
compose() {
    docker compose -p erp4us-preview --env-file "$ERP4US_ENV_FILE" \
        -f compose.yaml -f compose.delivery.yaml -f compose.manufacturing.yaml \
        -f compose.preview-identity.yaml -f compose.diagnostics.yaml "$@"
}
compose up -d preview-identity
python3 scripts/preview-identity.py configure
compose up --build -d
compose run --rm manufacturing-seed
compose run --rm master-data-seed
printf '%s\n' 'Preview API, broker and provider started. Start the Angular workspace with: cd ui && npm ci && npm start' \
    'Open http://127.0.0.1:4200/local-access. Demo credentials are in .erp4us-preview/credentials.json (local, private).'
