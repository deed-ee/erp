#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
python3 scripts/preview-identity.py prepare
export ERP4US_IDENTITY_DIR=./.erp4us-preview
export ERP4US_ENV_FILE="$PWD/.erp4us-preview/compose.env"
if [ ! -f "$ERP4US_ENV_FILE" ]; then sh scripts/init-dev.sh; fi
compose() {
    docker compose -p erp4us-preview --env-file "$ERP4US_ENV_FILE" \
        -f compose.yaml -f compose.delivery.yaml -f compose.manufacturing.yaml -f compose.preview-identity.yaml "$@"
}
compose up -d preview-identity
python3 scripts/preview-identity.py configure
compose up --build -d
compose run --rm manufacturing-seed
printf '%s\n' 'Preview API, broker and provider started. Start the Angular workspace with: cd ui && npm ci && npm start' \
    'Open http://127.0.0.1:4200/local-access. Demo credentials are in .erp4us-preview/credentials.json (local, private).'
