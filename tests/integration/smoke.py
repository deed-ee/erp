import json
import os
from pathlib import Path
import subprocess
import sys
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen
from uuid import uuid4


env = dict(line.split("=", 1) for line in Path(".env").read_text().splitlines()
           if line.strip() and not line.lstrip().startswith("#"))
base = f"http://127.0.0.1:{os.getenv('ERP4US_PORT', env.get('ERP4US_PORT', '8080'))}"
token = os.getenv("ERP4US_DEMO_TOKEN", env["ERP4US_DEMO_TOKEN"])


def request(path, data=None, bearer=token):
    headers = {"Accept": "application/json"}
    if bearer is not None:
        headers["Authorization"] = f"Bearer {bearer}"
    if data is not None:
        headers["Content-Type"] = "application/json"
    req = Request(base + path, data=data, headers=headers)
    try:
        response = urlopen(req, timeout=7)
    except HTTPError as exc:
        response = exc
    with response:
        body = response.read()
        return response.status, response.headers, json.loads(body) if body else None


def wait_until_ready():
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        try:
            if request("/health/ready", bearer=None)[0] == 204:
                return
        except (URLError, TimeoutError, ConnectionError):
            pass
        time.sleep(1)
    raise RuntimeError("Master Data did not become ready")


def main():
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
        command = [os.environ["COMPOSE_BIN"]] if "COMPOSE_BIN" in os.environ else ["docker", "compose"]
        subprocess.run([*command, "restart", "master-data"], check=True)
        wait_until_ready()
        assert request(path)[2] == body
    print("Master Data HTTP smoke passed" + (" (including restart)" if "--restart" in sys.argv else ""))


if __name__ == "__main__":
    main()
