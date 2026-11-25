import json
import os
from uuid import uuid4

from smoke import mongo_eval, request, wait_until_ready


def main():
    compose = [os.environ["COMPOSE_BIN"]] if "COMPOSE_BIN" in os.environ else ["docker", "compose"]
    wait_until_ready()
    path = "/v1/partners:register"
    partner = {"code": f"BP-{uuid4().hex[:16].upper()}",
               "name": "Example Trading", "roles": ["customer", "supplier"]}

    def register(body, **kwargs):
        return request(path, json.dumps(body).encode(), **kwargs)

    assert register(partner, bearer=None)[0] == 401
    for invalid in ({"roles": []}, {"roles": ["customer", "customer"]},
                    {"roles": ["other"]}, {"tenant_id": str(uuid4())},
                    {"name": "\u2000"}, {"code": "BP\n"}):
        assert register({**partner, **invalid})[0] == 422

    status, headers, created = register(partner)
    assert status == 201
    assert all(created[key] == value for key, value in partner.items())
    assert created["revision"] == created["schema_version"] == 1
    location = f"/v1/partners/{created['id']}"
    assert headers["Location"] == location
    assert request(location, bearer=None)[0] == 401
    assert request(location)[2] == created
    assert register({**partner, "roles": ["supplier"]})[0] == 409
    assert register({**partner, "code": f"BP-{uuid4().hex[:16].upper()}",
                     "roles": ["supplier"]})[0] == 201

    for field in ("tenant_id", "organization_id"):
        other_id = str(uuid4())
        fixture = {**created, "_id": other_id, field: str(uuid4())}
        del fixture["id"]
        try:
            assert mongo_eval(compose, f"print(db.partners.insertOne({json.dumps(fixture)}).acknowledged)")
            assert request(f"/v1/partners/{other_id}")[0] == 404
        finally:
            mongo_eval(compose, f"print(db.partners.deleteOne({{_id: {json.dumps(other_id)}}}).deletedCount)")
    print("Master Data business partner and scope checks passed")


if __name__ == "__main__":
    main()
