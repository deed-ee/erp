import base64
import json
import os
from pathlib import Path
import subprocess
from uuid import uuid4

from delivery import broker_probe, eventually
from smoke import env, mongo_eval


WIRE_HEADERS = {
    "protocolVersion": "Erp4us-Protocol-Version",
    "schemaVersion": "Erp4us-Schema-Version",
    "serviceVersion": "Erp4us-Service-Version",
    "messageId": "Erp4us-Message-Id",
    "correlationId": "Erp4us-Correlation-Id",
    "sourceService": "Erp4us-Source",
}


def wire_headers(envelope):
    return {"Content-Type": "application/json",
            **{header: envelope[field] for field, header in WIRE_HEADERS.items()}}


def query_snapshot(compose, example, *, check_invalid=False):
    inbox = f"_INBOX.article-config-reader.{uuid4().hex}"
    commands = [f"SUB {inbox}.* 1"]

    def publish(label, **overrides):
        message_id = str(uuid4())
        body = {**example, "messageId": message_id, "correlationId": message_id, **overrides}
        headers = wire_headers(body)
        if "tenantId" in body:
            headers["Erp4us-Tenant-Id"] = body["tenantId"]
        block = "\r\n".join(["NATS/1.0", *(f"{k}: {v}" for k, v in headers.items()), "", ""])
        payload = json.dumps(body, ensure_ascii=True, separators=(",", ":"))
        commands.extend([
            f"HPUB {example['messageType']} {inbox}.{label} {len(block)} {len(block) + len(payload)}",
            block + payload,
        ])

    if check_invalid:
        publish("protocol", protocolVersion="1.1.0")
        publish("schema", schemaVersion="1.1.0")
        publish("scope", tenantId=str(uuid4()), actor={"kind": "service", "id": example["sourceService"]})
    publish("valid")
    output = broker_probe(compose, "article_config_reader", env["DEV_NATS_CONFIG_READER_PASSWORD"],
                          commands, seconds=2)
    lines = output.splitlines()
    assert not any(line.startswith("-ERR") for line in lines), "Broker rejected the Python wire probe"
    replies = [(i, line.split()[1]) for i, line in enumerate(lines)
               if line.startswith(("MSG ", "HMSG "))]
    if not replies:
        return None
    assert [subject for _, subject in replies] == [f"{inbox}.valid"], "Unexpected settings reply"
    index, _ = replies[0]
    assert lines[index].startswith("HMSG "), "Expected Content-Type on the query reply"
    body_index = lines.index("", index + 1)
    assert lines[index + 1] == "NATS/1.0"
    assert "content-type: application/json" in [line.lower() for line in lines[index + 2:body_index]]
    return json.loads(lines[body_index + 1])["result"]


def main():
    compose = [os.environ["COMPOSE_BIN"]] if "COMPOSE_BIN" in os.environ else ["docker", "compose"]
    compose += ["-f", "compose.yaml", "-f", "compose.delivery.yaml"]
    fixture = Path(__file__).resolve().parents[2] / "contracts/examples/nats/relay-heartbeat-config-v1.json"
    examples = json.loads(fixture.read_text())
    get = examples["query"]["messageType"]
    changed = examples["changed"]["messageType"]
    heartbeat = "v1.diagnostics.system.article-delivery.event.service.heartbeat.v1"
    stream = "ERP4US_CONFIG_EVENTS_V1"

    def operator(*args, check=True):
        return subprocess.run([*compose, "run", "--rm", "--no-deps", *args],
                              capture_output=True, text=True, check=check, timeout=25)

    def cli_snapshot():
        result = operator("article-config-reader", check=False)
        return json.loads(result.stdout) if result.returncode == 0 else None

    def snapshot():
        return query_snapshot(compose, examples["query"])

    def journal(revision):
        return mongo_eval(compose, (
            "const row = db.article_relay_heartbeat_settings."
            f"findOne({{_id: NumberLong('{revision}')}});"
            "print(JSON.stringify(row && {...row, heartbeatSeconds: Number(row.heartbeatSeconds)}))"
        ))

    def probe(*commands, seconds=1):
        return broker_probe(compose, "provisioner", env["DEV_NATS_ADMIN_PASSWORD"],
                            commands, seconds=seconds)

    def last_notification():
        inbox = "_INBOX.config-test"
        lookup = json.dumps({"last_by_subj": changed})
        lines = probe(f"SUB {inbox} 1",
                      f"PUB $JS.API.STREAM.MSG.GET.{stream} {inbox} {len(lookup)}",
                      lookup).splitlines()
        frame = next(i for i, line in enumerate(lines) if line.startswith(f"MSG {inbox} "))
        return json.loads(lines[frame + 1])["message"]

    def heartbeats(seconds):
        return sum(line.startswith((f"MSG {heartbeat} ", f"HMSG {heartbeat} "))
                   for line in probe(f"SUB {heartbeat} 1", seconds=seconds).splitlines())

    initial = eventually(cli_snapshot, "startup configuration snapshot")
    assert initial == {"revision": 0, "heartbeatSeconds": 2}
    assert query_snapshot(compose, examples["query"], check_invalid=True) == initial
    assert heartbeats(4) >= 1
    other = get.replace(".system.", f".{uuid4()}.")
    output = broker_probe(compose, "article_config_reader", env["DEV_NATS_CONFIG_READER_PASSWORD"], [
        f"PUB {other} 2", "{}", f"PUB {changed} 2", "{}",
    ])
    for subject in [other, changed]:
        assert any("Permissions Violation" in line and subject in line for line in output.splitlines())
    output = broker_probe(compose, "article_receipts", env["DEV_NATS_RECEIPTS_PASSWORD"], [
        f"PUB {get} 2", "{}",
    ])
    assert any("Permissions Violation" in line and get in line for line in output.splitlines())

    # The local operator must be able to commit settings while NATS is down.
    subprocess.run([*compose, "stop", "nats"], check=True)
    try:
        operator("article-relay", "set-heartbeat", "0", "0")
        stored = journal(1)
        assert stored["heartbeatSeconds"] == 0 and "publishedAt" not in stored
        stale = operator("article-relay", "set-heartbeat", "0", "5", check=False)
        assert stale.returncode != 0 and "Stale heartbeat settings revision" in stale.stderr
        invalid = operator("article-relay", "set-heartbeat", "1", "3601", check=False)
        assert invalid.returncode != 0 and journal(2) is None
    finally:
        subprocess.run([*compose, "start", "nats"], check=True)

    eventually(lambda: journal(1).get("publishedAt"), "configuration publication acknowledgement")
    original = json.loads(stored["notification"])
    message = last_notification()
    assert message["subject"] == changed
    event = json.loads(base64.b64decode(message["data"]))
    assert event == original
    expected_event = examples["changed"]
    for field in ("messageType", "protocolVersion", "schemaVersion", "sourceService", "payload"):
        assert event[field] == expected_event[field]
    header_lines = base64.b64decode(message["hdrs"]).decode("ascii").splitlines()
    assert header_lines[0] == "NATS/1.0"
    headers = {name.lower(): value for name, value in
               (line.split(": ", 1) for line in header_lines[1:] if line)}
    for name, value in wire_headers(event).items():
        assert headers[name.lower()] == value
    assert headers["nats-msg-id"] == event["messageId"]
    assert not {"erp4us-tenant-id", "erp4us-organization-id"}.intersection(headers)
    assert heartbeats(5) == 0

    # A reader that missed the live event and a restarted owner still recover from storage.
    subprocess.run([*compose, "restart", "article-relay"], check=True)
    assert eventually(snapshot, "snapshot after relay restart") == {"revision": 1, "heartbeatSeconds": 0}
    operator("article-relay", "set-heartbeat", "1", "1")
    eventually(lambda: journal(2).get("publishedAt"), "reenable heartbeats")
    assert heartbeats(4) >= 2
    latest = last_notification()
    assert json.loads(base64.b64decode(latest["data"]))["payload"] == {"revision": 2}

    # Recreate an unrecorded ACK after JetStream's two-second duplicate window has passed.
    reset = mongo_eval(compose, (
        "print(JSON.stringify(db.article_relay_heartbeat_settings.updateOne("
        "{_id: NumberLong('1')}, {$unset: {publishedAt: ''}})))"
    ))
    assert reset["modifiedCount"] == 1
    eventually(lambda: journal(1).get("publishedAt"), "duplicate notification acknowledgement")
    retry = last_notification()
    assert retry["seq"] > latest["seq"]
    assert json.loads(base64.b64decode(retry["data"])) == original
    assert snapshot() == {"revision": 2, "heartbeatSeconds": 1}
    print("Python NATS wire, configuration ACL, heartbeat, restart and notification retry checks passed")


if __name__ == "__main__":
    main()
