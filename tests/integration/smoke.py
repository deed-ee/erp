import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryFile
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen
from uuid import uuid4


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


def verify_restore(compose, path, article, payload):
    target = [*compose, "-p", f"erp4us-restore-{uuid4().hex}"]
    port = os.getenv("ERP4US_RESTORE_PORT", "8081")
    target_env = {**os.environ, "ERP4US_PORT": port}

    def mongo_tool(project, tool):
        return [
            *project, "exec", "-T", "mongo", "sh", "-ec",
            f'exec {tool} --username "$MONGO_INITDB_ROOT_USERNAME" '
            '--password "$MONGO_INITDB_ROOT_PASSWORD" '
            '--authenticationDatabase admin',
        ]

    with TemporaryFile() as archive:
        subprocess.run(
            mongo_tool(compose, "mongodump --db master_data --archive"),
            stdout=archive, check=True,
        )
        archive.seek(0)
        try:
            subprocess.run(
                [*target, "up", "--wait", "--wait-timeout", "90", "mongo"],
                env=target_env, check=True,
            )
            subprocess.run(
                mongo_tool(target, "mongorestore --archive --stopOnError "
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
    print("Master Data HTTP smoke passed" + (" (including restart)" if "--restart" in sys.argv else ""))
    if "--restore" in sys.argv:
        verify_restore(compose, path, body, payload)


if __name__ == "__main__":
    main()
