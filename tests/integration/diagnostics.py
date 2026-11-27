"""Probe the real application endpoints before, during and after a Compose stop."""
import json
import os
import subprocess
from time import monotonic, sleep
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen
from uuid import uuid4

from delivery import broker_probe
from identity_fixture import token
from smoke import env, request


ORIGIN = f"http://127.0.0.1:{os.getenv('ERP4US_DIAGNOSTICS_PORT', '8091')}"
COMPOSE = ["docker", "compose", "-f", "compose.yaml", "-f", "compose.delivery.yaml",
           "-f", "compose.manufacturing.yaml", "-f", "compose.diagnostics.yaml"]


def query(credential=None):
    headers = {} if credential is None else {"Authorization": f"Bearer {credential}"}
    try:
        with urlopen(Request(ORIGIN + "/v1/admin/application-health", headers=headers), timeout=8) as response:
            assert response.headers["Cache-Control"] == "no-store"
            return response.status, json.load(response)
    except HTTPError as error:
        return error.status, None


def wait(credential, desired, matches=lambda _: True):
    deadline = monotonic() + 30
    while monotonic() < deadline:
        try:
            status, report = query(credential)
        except URLError:
            sleep(0.5)
            continue
        assert status == 200
        states = {entry["service"]: entry["readiness"] for entry in report["application_checks"]}
        if all(states[name] == state for name, state in desired.items()) and matches(report):
            return report
        sleep(0.5)
    raise RuntimeError(f"Application checks did not transition to {desired}")


def check_broker_permissions():
    subjects = (
        "$JS.API.STREAM.MSG.GET.ERP4US_BUSINESS_EVENTS_V1",
        f"$JS.API.STREAM.DELETE.DIAG_NOT_A_REAL_STREAM_{uuid4().hex}",
        "$JS.API.CONSUMER.MSG.NEXT.ERP4US_BUSINESS_EVENTS_V1.article-receipts",
    )
    # Test the delete ACL without targeting a real stream.
    output = broker_probe(COMPOSE, "platform_diagnostics", env["DEV_NATS_DIAGNOSTICS_PASSWORD"], [
        "SUB _INBOX.article-consumer.> 1",
        *(command for subject in subjects for command in (f"PUB {subject} 1", "!")),
    ])
    for denied in (*subjects, "_INBOX.article-consumer.>"):
        assert any("Permissions Violation" in line and denied in line for line in output.splitlines()), denied


if __name__ == "__main__":
    credential = token(subject="synthetic-platform")
    before = wait(credential, {"master-data": "ready", "manufacturing": "ready", "jetstream": "ready"},
                  lambda report: all(value is not None for value in report["jetstream"].values()))
    assert query()[0] == 401
    assert query(token())[0] == 403  # Tenant writer is not a platform monitor.
    assert request("/v1/partners", bearer=credential)[0] == 403
    check_broker_permissions()
    count = before["jetstream"]["stream"]["messages"]
    article = json.dumps({"sku": f"DIAG-{uuid4().hex[:12].upper()}", "name": "Diagnostic fixture"}).encode()
    assert request("/v1/articles:register", article)[0] == 201
    wait(credential, {"jetstream": "ready"},
         lambda report: (report["jetstream"]["stream"] or {}).get("messages", 0) > count)
    try:
        subprocess.run([*COMPOSE, "stop", "manufacturing"], check=True, timeout=40)
        down = wait(credential, {"manufacturing": "unreachable", "master-data": "ready"})
        assert down["observed_at_unix_ms"] > before["observed_at_unix_ms"]
    finally:
        subprocess.run([*COMPOSE, "start", "manufacturing"], check=True, timeout=40)
    wait(credential, {"manufacturing": "ready"})
    try:
        subprocess.run([*COMPOSE, "stop", "nats"], check=True, timeout=40)
        down = wait(credential, {"jetstream": "unreachable", "manufacturing": "ready"})
        assert down["jetstream"] == {"stream": None, "consumer": None}
    finally:
        subprocess.run([*COMPOSE, "start", "nats"], check=True, timeout=40)
    wait(credential, {"jetstream": "ready"},
         lambda report: (report["jetstream"]["stream"] or {}).get("messages", 0) > count)
    print("Platform-only readiness, live counters, broker ACLs, and stop/recovery checks passed")
