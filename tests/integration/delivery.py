import json
import os
import subprocess
import time
from uuid import uuid4

from smoke import env, mongo_eval, request, wait_until_ready


def broker_probe(compose, user, password, commands, seconds=1):
    identity = json.dumps({"user": user, "pass": password, "headers": True})
    packet = f"CONNECT {identity}\r\n" + "\r\n".join([*commands, "PING", ""])
    output = subprocess.run(
        [*compose, "exec", "-T", "nats", "sh", "-c",
         '(cat; sleep "$1") | timeout "$1" nc 127.0.0.1 4222', "probe", str(seconds)],
        input=packet, capture_output=True, text=True, timeout=seconds + 10,
    )
    assert "PONG" in output.stdout, "NATS probe did not complete its handshake"
    return output.stdout


def check_broker_permissions(compose, tenant):
    other = f"v1.business.{uuid4()}.master-data.event.article.registered.v1"
    config = f"v1.config.{tenant}.master-data.query.settings.get.v1"
    heartbeat = "v1.diagnostics.system.article-delivery.event.service.heartbeat.v1"

    output = broker_probe(compose, "article_relay", env["DEV_NATS_RELAY_PASSWORD"], [
        f"PUB {other} 2", "{}", f"PUB {config} 2", "{}",
        "SUB _INBOX.article-consumer.> 1", "PUB $JS.API.STREAM.LIST 2", "{}",
    ])
    for denied in [other, config, "_INBOX.article-consumer.>", "$JS.API.STREAM.LIST"]:
        assert any("Permissions Violation" in line and denied in line for line in output.splitlines()), denied

    output = broker_probe(compose, "provisioner", env["DEV_NATS_ADMIN_PASSWORD"],
                          [f"SUB {heartbeat} 1"], seconds=18)
    assert any(line.startswith(f"HMSG {heartbeat} ") for line in output.splitlines())
    assert "Erp4us-Protocol-Version: 1.0.0" in output
    assert '"nats":"connected"' in output
    print("NATS tenant, layer, inbox and administration permissions; Core heartbeat checks passed")


def eventually(query, description):
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        if result := query():
            return result
        time.sleep(0.5)
    raise AssertionError(f"Timed out waiting for {description}")


def register():
    payload = json.dumps({"sku": f"DELIVERY-{uuid4().hex[:16].upper()}",
                          "name": "Delivery test article"}).encode()
    status, headers, article = request("/v1/articles:register", payload)
    assert status == 201
    return {**article, "traceparent": headers["traceparent"]}


def main():
    compose = ([os.environ["COMPOSE_BIN"]] if "COMPOSE_BIN" in os.environ else
               ["docker", "compose"])
    compose += ["-f", "compose.yaml", "-f", "compose.delivery.yaml"]

    def run(*args):
        subprocess.run([*compose, *args], check=True)

    def event(article):
        return mongo_eval(compose, "print(JSON.stringify(db.outbox.findOne("
                          f"{{'payload.articleId': {json.dumps(article['id'])}}})))")

    def receipts(article):
        return mongo_eval(compose, "print(JSON.stringify(db.getSiblingDB('article_receipts')"
                          f".receipts.find({{articleId: {json.dumps(article['id'])}}}).toArray()))")

    wait_until_ready()
    run("stop", "nats")
    try:
        article = register()
        stored = event(article)
        assert "publishedAt" not in stored
        assert receipts(article) == []
        run("restart", "article-relay")
    finally:
        run("start", "nats")
    first = eventually(lambda: receipts(article), "receipt after broker restart")
    message_id = stored["messageId"]
    assert first == [{
        "_id": message_id, "articleId": article["id"], "revision": article["revision"],
        "tenantId": article["tenant_id"], "organizationId": article["organization_id"],
        "correlationId": stored["correlationId"],
        "traceparent": article["traceparent"],
    }]
    eventually(lambda: event(article).get("publishedAt"), "broker acknowledgement")
    check_broker_permissions(compose, article["tenant_id"])

    # Recreate the state left by a relay crash between broker ack and Mongo update.
    # Wait beyond the stream's duplicate window so the consumer must deduplicate.
    run("restart", "article-receipts")
    time.sleep(3)
    changed = mongo_eval(
        compose, "print(db.outbox.updateOne("
        f"{{messageId: {json.dumps(message_id)}}}, {{$unset: {{publishedAt: 1}}}}).modifiedCount)",
    )
    assert changed == 1
    eventually(lambda: event(article).get("publishedAt"), "republished event")

    def quarantined():
        return mongo_eval(compose, "print(db.getSiblingDB('article_receipts')"
                          ".quarantine.countDocuments({reason: 'invalid ArticleRegistered envelope'}))")

    previous_quarantined = quarantined()
    malformed = {key: value for key, value in stored.items() if key != "_id"}
    malformed["messageId"] = str(uuid4())
    malformed["payload"] = {"articleId": article["id"], "revision": 2}
    mongo_eval(compose, f"print(db.outbox.insertOne({json.dumps(malformed)}).acknowledged)")
    eventually(
        lambda: mongo_eval(compose, "print(db.outbox.countDocuments("
                           f"{{messageId: {json.dumps(malformed['messageId'])}, "
                           "deliveryRejectedAt: {$exists: true}, publishedAt: {$exists: false}}))"),
        "invalid outbox entry rejected without publication",
    )
    subject = f"v1.business.{article['tenant_id']}.master-data.event.article.registered.v1"
    response = broker_probe(compose, "article_relay", env["DEV_NATS_RELAY_PASSWORD"], [
        "SUB _INBOX.article-relay.smoke 1",
        f"PUB {subject} _INBOX.article-relay.smoke 2", "{}",
    ])
    assert '"stream":"ERP4US_BUSINESS_EVENTS_V1"' in response
    eventually(lambda: quarantined() == previous_quarantined + 1, "quarantine receipt")

    followup = register()
    eventually(lambda: receipts(followup), "receipt after replay")
    assert receipts(article) == first
    print("JetStream outage, restart, duplicate delivery and quarantine checks passed")

    # Reject only the probe's receipt so the rest of the pipeline can still drain.
    run("stop", "article-receipts")
    probe = register()
    failed = event(probe)
    db = "db.getSiblingDB('article_receipts')"

    def set_validator(validator):
        result = mongo_eval(compose, f"print({db}.runCommand({{collMod: 'receipts', "
                            f"validator: {json.dumps(validator)}}}).ok)")
        assert result == 1

    original = mongo_eval(compose, f"print(JSON.stringify({db}.getCollectionInfos("
                          "{name: 'receipts'})[0].options.validator || {}))")
    try:
        set_validator({"articleId": {"$ne": probe["id"]}})
        run("start", "article-receipts")
        rejected = eventually(
            lambda: mongo_eval(compose, f"const entry = {db}.quarantine.findOne("
                               f"{{messageId: {json.dumps(failed['messageId'])}}}); "
                               "print(JSON.stringify(entry && {...entry, "
                               "deliveryCount: Number(entry.deliveryCount)}))"),
            "quarantine after receipt retries",
        )
        assert {key: value for key, value in rejected.items() if key != "_id"} == {
            "reason": "receipt delivery limit reached",
            "messageId": failed["messageId"],
            "correlationId": failed["correlationId"],
            "tenantId": probe["tenant_id"],
            "organizationId": probe["organization_id"],
            "deliveryCount": 4,
            "traceparent": probe["traceparent"],
        }, rejected
        assert receipts(probe) == []
        healthy = register()
        eventually(lambda: receipts(healthy), "receipt after retry exhaustion")
    finally:
        set_validator(original)
        run("start", "article-receipts")
    print("JetStream receipt retry exhaustion check passed")

    def replay(sequence):
        return subprocess.run(
            [*compose, "run", "--rm", "--no-deps", "article-replay", "replay", sequence],
            capture_output=True, text=True,
        )

    denied = replay("999999999999")
    assert denied.returncode != 0
    assert "No retry-exhausted quarantine entry" in denied.stderr
    stream, sequence = rejected["_id"].split(":")
    assert stream == "ERP4US_BUSINESS_EVENTS_V1"
    accepted = replay(sequence)
    assert accepted.returncode == 0, accepted.stderr
    recovered = eventually(lambda: receipts(probe), "receipt after operator replay")
    assert len(recovered) == 1 and recovered[0]["_id"] == failed["messageId"]
    assert recovered[0]["traceparent"] == probe["traceparent"]
    audit = mongo_eval(compose, f"print(JSON.stringify({db}.quarantine.findOne("
                       f"{{_id: {json.dumps(rejected['_id'])}}})))")
    assert audit["replayRequestedAt"] and audit["replayedAt"]
    assert int(audit["replaySequence"]) > int(sequence)
    repeated = replay(sequence)
    assert repeated.returncode != 0 and "already been replayed" in repeated.stderr
    assert receipts(probe) == recovered
    print("JetStream operator replay and HTTP trace correlation checks passed")


if __name__ == "__main__":
    main()
