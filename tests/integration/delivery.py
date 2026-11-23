import json
import os
import subprocess
import time
from uuid import uuid4

from smoke import mongo_eval, request, wait_until_ready


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
    status, _, article = request("/v1/articles:register", payload)
    assert status == 201
    return article


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
    }]
    eventually(lambda: event(article).get("publishedAt"), "broker acknowledgement")

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
    eventually(lambda: quarantined() == previous_quarantined + 1, "quarantine receipt")

    followup = register()
    eventually(lambda: receipts(followup), "receipt after replay")
    assert receipts(article) == first
    print("JetStream outage, restart, duplicate delivery and quarantine checks passed")


if __name__ == "__main__":
    main()
