import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryFile
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen
from uuid import UUID, uuid4


env = dict(line.split("=", 1) for line in Path(".env").read_text().splitlines()
           if line.strip() and not line.lstrip().startswith("#"))
base = f"http://127.0.0.1:{os.getenv('ERP4US_PORT', env.get('ERP4US_PORT', '8080'))}"
token = os.getenv("ERP4US_DEMO_TOKEN", env["ERP4US_DEMO_TOKEN"])


def request(path, data=None, bearer=token, *, origin=base):
    headers = {"Accept": "application/json"}
    if bearer is not None:
        headers["Authorization"] = f"Bearer {bearer}"
    if data is not None:
        headers["Content-Type"] = "application/json"
    req = Request(origin + path, data=data, headers=headers)
    try:
        response = urlopen(req, timeout=7)
    except HTTPError as exc:
        response = exc
    with response:
        body = response.read()
        return response.status, response.headers, json.loads(body) if body else None


def wait_until_ready(origin=base):
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        try:
            if request("/health/ready", bearer=None, origin=origin)[0] == 204:
                return
        except (URLError, TimeoutError, ConnectionError):
            pass
        time.sleep(1)
    raise RuntimeError(f"Master Data at {origin} did not become ready")


def mongo_tool(project, tool, *args):
    return [
        *project, "exec", "-T", "mongo", "sh", "-ec",
        'tool=$1; shift; exec "$tool" --username "$MONGO_INITDB_ROOT_USERNAME" '
        '--password "$MONGO_INITDB_ROOT_PASSWORD" '
        '--authenticationDatabase admin "$@"', "mongo-tool", tool, *args,
    ]


def mongo_eval(compose, code):
    command = mongo_tool(
        compose, "mongosh", "--quiet", "--norc",
        "mongodb://mongo:27017/master_data?replicaSet=erp4us", "--eval", code,
    )
    return json.loads(subprocess.check_output(command, text=True))


def verify_outbox(compose, article):
    events = mongo_eval(
        compose, f"print(JSON.stringify(db.outbox.find({{'payload.articleId': "
        f"{json.dumps(article['id'])}}}, {{_id: 0}}).toArray()))",
    )
    assert len(events) == 1
    event = events[0]
    assert str(UUID(event["messageId"])) == event["messageId"]
    assert event == {
        "messageId": event["messageId"],
        "messageType": "business.master-data.article.registered.v1",
        "schemaVersion": 1,
        "timestamp": article["created_at"],
        "tenantId": article["tenant_id"],
        "organizationId": article["organization_id"],
        "actor": {"kind": "user", "id": article["created_by"]},
        "correlationId": event["messageId"],
        "sourceService": "erp4us.master-data",
        "payload": {"articleId": article["id"], "revision": article["revision"]},
    }

    sku = f"ROLLBACK-{uuid4().hex[:16].upper()}"
    payload = json.dumps({"sku": sku, "name": "Rollback probe"}).encode()
    before = mongo_eval(compose, "print(db.outbox.countDocuments({}))")
    validator = mongo_eval(
        compose, "print(JSON.stringify(db.getCollectionInfos({name: 'outbox'})[0]"
        ".options.validator || {}))",
    )
    try:
        configure_outbox_validator(compose, {"messageId": {"$exists": False}})
        assert request("/v1/articles:register", payload)[0] == 503
        assert mongo_eval(
            compose, f"print(db.articles.countDocuments({{sku: {json.dumps(sku)}}}))",
        ) == 0
        assert mongo_eval(compose, "print(db.outbox.countDocuments({}))") == before
    finally:
        configure_outbox_validator(compose, validator)
    status, _, created = request("/v1/articles:register", payload)
    assert status == 201
    assert mongo_eval(
        compose, f"print(db.outbox.countDocuments({{'payload.articleId': {json.dumps(created['id'])}}}))",
    ) == 1
    print("Master Data transactional outbox passed")


def configure_outbox_validator(compose, validator):
    if mongo_eval(compose, "print(db.runCommand({collMod: 'outbox', "
                  f"validator: {json.dumps(validator)}}}).ok)") != 1:
        raise RuntimeError("Could not configure the outbox validator")


def verify_restore(compose, path, article, payload):
    target = [*compose, "-p", f"erp4us-restore-{uuid4().hex}"]
    port = os.getenv("ERP4US_RESTORE_PORT", "8081")
    target_env = {**os.environ, "ERP4US_PORT": port}

    with TemporaryFile() as archive:
        subprocess.run(
            mongo_tool(compose, "mongodump", "--db", "master_data", "--archive"),
            stdout=archive, check=True,
        )
        archive.seek(0)
        try:
            subprocess.run(
                [*target, "up", "--wait", "--wait-timeout", "90", "mongo", "mongo-init"],
                env=target_env, check=True,
            )
            subprocess.run(
                mongo_tool(target, "mongorestore", "--archive", "--stopOnError",
                           "--nsInclude=master_data.*"),
                stdin=archive, env=target_env, check=True,
            )
            subprocess.run(
                [*target, "up", "--build", "-d", "master-data"],
                env=target_env, check=True,
            )
            origin = f"http://127.0.0.1:{port}"
            wait_until_ready(origin)
            status, _, restored = request(path, origin=origin)
            assert status == 200 and restored == article
            assert request("/v1/articles:register", payload, origin=origin)[0] == 409
            print("Master Data backup and restore passed")
        finally:
            subprocess.run([*target, "down", "-v", "--rmi", "local"], env=target_env, check=True)


def main():
    compose = [os.environ["COMPOSE_BIN"]] if "COMPOSE_BIN" in os.environ else ["docker", "compose"]
    wait_until_ready()
    assert request("/health/live", bearer=None)[0] == 204
    collection = "/v1/articles:register"
    article = {"sku": f"BRACKET-{uuid4().hex[:16].upper()}", "name": "Steel bracket"}
    payload = json.dumps(article).encode()
    for unauthorized in (None, "0" * 64):
        status, headers, _ = request(collection, payload, bearer=unauthorized)
        assert status == 401 and headers["WWW-Authenticate"] == "Bearer"
    assert request(collection, b"{")[0] == 400
    assert request(collection, b" " * 4097)[0] == 413
    assert request(collection, b'{"sku":"invalid lowercase","name":"x"}')[0] == 422
    assert request(collection, json.dumps({**article, "tenant_id": str(uuid4())}).encode())[0] == 422
    status, headers, body = request(collection, payload)
    assert status == 201 and body["sku"] == article["sku"] and body["name"] == article["name"]
    assert body["revision"] == body["schema_version"] == 1
    assert "_id" not in body
    path = f"/v1/articles/{body['id']}"
    assert headers["Location"] == path and headers["Cache-Control"] == "no-store"
    assert request(path, bearer=None)[0] == 401
    assert request(path)[2] == body
    assert request(f"/v1/articles/{uuid4()}")[0] == 404
    status, headers, problem = request(collection, payload)
    assert status == problem["status"] == 409
    assert headers.get_content_type() == "application/problem+json"
    if "--restart" in sys.argv:
        subprocess.run([*compose, "restart", "master-data"], check=True)
        wait_until_ready()
        assert request(path)[2] == body
    if "--outbox" in sys.argv:
        verify_outbox(compose, body)
    print("Master Data HTTP smoke passed" + (" (including restart)" if "--restart" in sys.argv else ""))
    if "--restore" in sys.argv:
        verify_restore(compose, path, body, payload)


if __name__ == "__main__":
    main()
