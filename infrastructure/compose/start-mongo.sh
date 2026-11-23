#!/bin/sh
set -eu
umask 077
keyfile=$(mktemp /tmp/erp4us-mongo-key.XXXXXX)
openssl rand -base64 48 > "$keyfile"
chown mongodb:mongodb "$keyfile"
exec docker-entrypoint.sh mongod --replSet erp4us --keyFile "$keyfile"
