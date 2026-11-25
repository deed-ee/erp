import base64
import json
import os
import subprocess
from uuid import uuid4

from delivery import broker_probe, eventually
from smoke import env, mongo_eval


def main():
    compose = [os.environ["COMPOSE_BIN"]] if "COMPOSE_BIN" in os.environ else ["docker", "compose"]
    compose += ["-f", "compose.yaml", "-f", "compose.delivery.yaml"]
    get = "v1.config.system.article-delivery.query.heartbeat.get.v1"
    changed = "v1.config.system.article-delivery.event.heartbeat.changed.v1"
    heartbeat = "v1.diagnostics.system.article-delivery.event.service.heartbeat.v1"
    stream = "ERP4US_CONFIG_EVENTS_V1"

    def operator(*args, check=True):
        return subprocess.run([*compose, "run", "--rm", "--no-deps", *args],
                              capture_output=True, text=True, check=check, timeout=25)

    def snapshot():
        result = operator("article-config-reader", check=False)
        return json.loads(result.stdout) if result.returncode == 0 else None

    def journal(revision):
        return mongo_eval(compose, (
            "print(JSON.stringify(db.article_relay_heartbeat_settings."
            f"findOne({{_id: NumberLong('{revision}')}})))"
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

    initial = eventually(snapshot, "startup configuration snapshot")
    assert initial == {"revision": 0, "heartbeatSeconds": 2}
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
    assert json.loads(base64.b64decode(message["data"])) == original
    assert original["payload"] == {"revision": 1}
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
    print("Configuration ACL, stale revision, heartbeat, restart and notification retry checks passed")


if __name__ == "__main__":
    main()
