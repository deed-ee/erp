#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
umask 077
set -C
root_password=$(openssl rand -hex 32)
service_password=$(openssl rand -hex 32)
{
    printf 'DEV_MONGO_ROOT_PASSWORD=%s\n' "$root_password"
    printf 'DEV_MASTER_DATA_PASSWORD=%s\n' "$service_password"
} > .env
printf 'Created .env with local development credentials.\n'
