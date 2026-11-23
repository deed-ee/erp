import json
from pathlib import Path
from uuid import uuid4

from jsonschema import Draft202012Validator, FormatChecker
from openapi_spec_validator import validate_url
from referencing import Registry, Resource
import yaml


ROOT = Path(__file__).resolve().parents[2] / "contracts"
OPENAPI = ROOT / "openapi/master-data-v1.yaml"


def load(path):
    return json.loads(path.read_text())


schemas = {}
for path in (ROOT / "schemas").glob("*.json"):
    schema = load(path)
    Draft202012Validator.check_schema(schema)
    schemas[path] = schema
registry = Registry().with_resources((path.as_uri(), Resource.from_contents(data))
                                     for path, data in schemas.items())


def validator(name):
    path = ROOT / "schemas" / name
    return Draft202012Validator({"$ref": path.as_uri()}, registry=registry,
                                format_checker=FormatChecker())


validate_url(OPENAPI.as_uri())
manifest_path = ROOT / "manifests/master-data-v1.json"
manifest = load(manifest_path)
validator("service-manifest-v1.json").validate(manifest)
assert (manifest_path.parent / manifest["openapi"]).resolve() == OPENAPI
api = yaml.safe_load(OPENAPI.read_text())
operations = {op["operationId"]: method for path in api["paths"].values()
              for method, op in path.items() if isinstance(op, dict) and "operationId" in op}
for ref in manifest["entities"]:
    entity_path = (manifest_path.parent / ref).resolve()
    entity = load(entity_path)
    validator("semantic-entity-v1.json").validate(entity)
    assert (entity_path.parent / entity["schema"]).resolve() in schemas
    for operation in entity["operations"].values():
        assert operations[operation["operationId"]] == {"command": "post", "query": "get"}[operation["kind"]]
for path in manifest["health"].values():
    assert api["paths"][path]["get"]["security"] == []

command = validator("register-article-v1.json")
command.validate({"sku": "BRACKET-1", "name": "Steel bracket"})
for bad in ({"sku": "A\n", "name": "Name"}, {"sku": "A", "name": "\u0085"},
            {"sku": "A", "name": " ", "tenant_id": str(uuid4())}):
    assert not command.is_valid(bad)
validator("article-v1.json").validate({
    "id": str(uuid4()), "tenant_id": str(uuid4()), "organization_id": str(uuid4()),
    "created_by": str(uuid4()), "schema_version": 1, "revision": 1,
    "created_at": "2026-11-23T12:00:00Z", "sku": "BRACKET-1", "name": "Steel bracket",
})
envelope = validator("event-envelope-v1.json")
event = {
    "messageId": str(uuid4()), "messageType": "business.sales.order.confirmed.v1",
    "schemaVersion": 1, "timestamp": "2026-11-23T12:00:00Z", "tenantId": str(uuid4()),
    "actor": {"kind": "service", "id": "erp4us.sales"}, "correlationId": str(uuid4()),
    "sourceService": "erp4us.sales", "payload": {"orderId": str(uuid4())},
}
envelope.validate(event)
assert not envelope.is_valid({**event, "tenantId": "not-a-uuid"})
print("OpenAPI, JSON Schemas and protocol examples validated")
