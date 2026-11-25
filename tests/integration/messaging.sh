#!/bin/sh
set -eu

# Bind a random port on loopback only. Always remove exactly the test container.
if [ -n "${COMPOSE_BIN:-}" ]; then
    set -- "$COMPOSE_BIN"
else
    set -- docker compose
fi
image=$("$@" -f compose.yaml -f compose.delivery.yaml config --images nats)
container=$(docker run -d --rm -p 127.0.0.1::4222 "$image" -js)
trap 'docker stop "$container" >/dev/null' EXIT
address=$(docker port "$container" 4222/tcp)
NATS_TEST_URL="nats://$address" cargo test --locked -p erp4us-messaging --test broker -- --ignored
