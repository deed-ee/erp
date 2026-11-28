#!/bin/sh
# Exercise presentation with disposable credentials on loopback only.
set -eu
umask 077
root=$(CDPATH= cd "$(dirname "$0")/../.." && pwd)
theme="$root/infrastructure/keycloak/themes/erp4us-v1"
cmp "$theme/login/resources/img/erp4us-symbol.svg" "$root/ui/public/brand/symbol.svg"

image=quay.io/keycloak/keycloak:26.8.2@sha256:7e66a96e4739239070aeaf72e4c5decf0f7f682cfc725b9f7ea2b336eaba38b3
port=${ERP4US_THEME_PORT:-19443}
callback_port=${ERP4US_THEME_CALLBACK_PORT:-19444}
for candidate in "$port" "$callback_port"; do
    case "$candidate" in ''|*[!0-9]*) exit 2 ;; esac
    [ "$candidate" -ge 1024 ] && [ "$candidate" -le 65535 ] || exit 2
done
[ "$port" -ne "$callback_port" ] || exit 2
origin="https://127.0.0.1:$port"
fixture=$(mktemp -d "${TMPDIR:-/tmp}/erp4us-theme.XXXXXX")
container=
cleanup() {
    if [ -n "$container" ]; then docker rm -f "$container" >/dev/null; fi
    # Only remove the three files created in this test, then the private directory.
    rm -f "$fixture/tls.crt" "$fixture/tls.key" "$fixture/theme-check-realm.json"
    rmdir "$fixture"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
openssl req -x509 -newkey rsa:2048 -nodes -days 1 -subj '/CN=127.0.0.1' \
    -addext 'subjectAltName=IP:127.0.0.1' -keyout "$fixture/tls.key" -out "$fixture/tls.crt" 2>/dev/null

export ERP4US_THEME_ORIGIN="$origin"
export ERP4US_THEME_CALLBACK_PORT="$callback_port"
export ERP4US_THEME_PASSWORD="Theme-$(openssl rand -hex 20)"
python3 - "$fixture/theme-check-realm.json" <<'PY'
import json
import os
import sys
from pathlib import Path


def user(username, temporary=False):
    return {
        "username": username, "enabled": True, "firstName": "Theme", "lastName": "Check",
        "email": f"{username}@example.test",
        "requiredActions": ["UPDATE_PASSWORD"] if temporary else [],
        "credentials": [{"type": "password", "value": os.environ["ERP4US_THEME_PASSWORD"]}],
    }


Path(sys.argv[1]).write_text(json.dumps({
    "realm": "theme-check", "enabled": True, "displayName": "erp4us",
    "loginTheme": "erp4us-v1", "internationalizationEnabled": True,
    "supportedLocales": ["en", "de"], "defaultLocale": "en", "resetPasswordAllowed": True,
    "clients": [{
        "clientId": "theme-browser", "publicClient": True, "standardFlowEnabled": True,
        "directAccessGrantsEnabled": False,
        "redirectUris": [f"http://127.0.0.1:{os.environ['ERP4US_THEME_CALLBACK_PORT']}/return"],
        "attributes": {"pkce.code.challenge.method": "S256"},
    }],
    "users": [user("theme-reader"), user("theme-update", temporary=True)],
}) + "\n")
PY
# The private host directory is never mounted; only these disposable files are.
chmod 644 "$fixture/tls.key" "$fixture/tls.crt" "$fixture/theme-check-realm.json"
container=$(docker run -d --pull never \
    --cap-drop ALL --security-opt no-new-privileges:true --memory 768m --cpus 2 \
    -e 'JAVA_OPTS_KC_HEAP=-Xms96m -Xmx384m' \
    -p "127.0.0.1:$port:8443" \
    -v "$theme:/opt/keycloak/themes/erp4us-v1:ro" \
    -v "$fixture/tls.crt:/opt/keycloak/theme-check.crt:ro" \
    -v "$fixture/tls.key:/opt/keycloak/theme-check.key:ro" \
    -v "$fixture/theme-check-realm.json:/opt/keycloak/data/import/theme-check-realm.json:ro" \
    "$image" start-dev --import-realm --hostname="$origin" --http-enabled=false \
    --https-certificate-file=/opt/keycloak/theme-check.crt --https-certificate-key-file=/opt/keycloak/theme-check.key)

python3 - "$origin" "$fixture/tls.crt" <<'PY'
import ssl
import sys
import time
from urllib.error import URLError
from urllib.request import urlopen

origin, certificate = sys.argv[1:]
context = ssl.create_default_context(cafile=certificate)
deadline = time.monotonic() + 90
while time.monotonic() < deadline:
    try:
        with urlopen(origin + "/realms/theme-check/.well-known/openid-configuration",
                     context=context, timeout=2) as response:
            if response.status == 200:
                break
    except (URLError, TimeoutError, ConnectionError):
        pass
    time.sleep(1)
else:
    raise SystemExit("Disposable Keycloak realm did not become ready")
PY

cd "$root/ui"
./node_modules/.bin/playwright test --config playwright.keycloak-theme.config.ts
