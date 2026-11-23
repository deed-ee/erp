import json
import os
import subprocess
import sys
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen
from uuid import uuid4
from identity_fixture import (ACTOR, EXTERNAL_ORGANIZATION, ORGANIZATION, OTHER_ORGANIZATION,
                              OTHER_TENANT, TENANT, token)


base = f"http://127.0.0.1:{os.getenv('ERP4US_PORT', '8080')}"


def request(path, data=None, bearer=None, scope=(TENANT, ORGANIZATION)):
    headers = {"Accept": "application/json"}
    if bearer is not None:
        headers["Authorization"] = f"Bearer {bearer}"
    if scope is not None:
        headers["X-Erp4us-Tenant"], headers["X-Erp4us-Organization"] = scope
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
    writer = token()
    reader = token(subject="synthetic-reader", header_type="JWT", typ="Bearer")
    separate_writer = token(subject="synthetic-registration")
    signed_data, signature = writer.rsplit(".", 1)
    altered_signature = ("B" if signature[0] == "A" else "A") + signature[1:]
    for unauthorized in (None, "not-a-token", token(iss="https://other.invalid"),
                         token(aud="different-api"), token(exp=int(time.time()) - 1),
                         token(nbf=int(time.time()) + 600), token(header_type="JWT", typ="ID"),
                         f"{signed_data}.{altered_signature}"):
        status, headers, _ = request(collection, payload, bearer=unauthorized)
        assert status == 401 and headers["WWW-Authenticate"] == "Bearer"
    assert request(collection, payload, writer, None)[0] == 400
    assert request(collection, payload, writer, ("not-a-uuid", ORGANIZATION))[0] == 400
    assert request(collection, payload, token(subject="unbound"))[0] == 403
    assert request(collection, payload, reader)[0] == 403
    assert request(collection, b"{", writer)[0] == 400
    assert request(collection, b" " * 4097, writer)[0] == 413
    assert request(collection, b'{"sku":"invalid lowercase","name":"x"}', writer)[0] == 422
    assert request(collection, json.dumps({**article, "tenant_id": str(uuid4())}).encode(), writer)[0] == 422
    status, headers, body = request(collection, payload, writer)
    assert status == 201 and body["sku"] == article["sku"] and body["name"] == article["name"]
    assert body["revision"] == body["schema_version"] == 1
    assert (body["tenant_id"], body["organization_id"], body["created_by"]) == (
        TENANT, ORGANIZATION, ACTOR
    )
    assert "_id" not in body
    path = f"/v1/articles/{body['id']}"
    assert headers["Location"] == path and headers["Cache-Control"] == "no-store"
    assert request(path, bearer=None)[0] == 401
    assert request(path, bearer=reader)[2] == body
    assert request(path, bearer=separate_writer)[0] == 403
    assert request(f"/v1/articles/{uuid4()}", bearer=writer)[0] == 404
    assert request(path, bearer=writer, scope=(OTHER_TENANT, ORGANIZATION))[0] == 403
    for scope in ((TENANT, OTHER_ORGANIZATION), (OTHER_TENANT, EXTERNAL_ORGANIZATION)):
        assert request(path, bearer=reader, scope=scope)[0] == 403
        assert request(path, bearer=writer, scope=scope)[0] == 404
        assert request(collection, payload, writer, scope)[0] == 403
        status, _, sibling = request(collection, payload, separate_writer, scope)
        assert status == 201 and tuple(sibling[key] for key in ("tenant_id", "organization_id")) == scope
        assert request(f"/v1/articles/{sibling['id']}", bearer=writer)[0] == 404
    status, headers, problem = request(collection, payload, writer)
    assert status == problem["status"] == 409
    assert headers.get_content_type() == "application/problem+json"
    if "--restart" in sys.argv:
        command = [os.environ["COMPOSE_BIN"]] if "COMPOSE_BIN" in os.environ else ["docker", "compose"]
        subprocess.run([*command, "restart", "master-data"], check=True)
        wait_until_ready()
        assert request(path, bearer=token())[2] == body
    print("Master Data HTTP smoke passed" + (" (including restart)" if "--restart" in sys.argv else ""))


if __name__ == "__main__":
    main()
