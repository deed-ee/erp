import json
from copy import deepcopy
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


def check_article_reader(api, response_schema, previous):
    for expected in previous["operations"]:
        path, method = expected["path"], expected["method"]
        operation = api["paths"].get(path, {}).get(method, {})
        if operation.get("operationId") != expected["operationId"]:
            raise AssertionError(f"Article v1 reader lost {method.upper()} {path}")
        if method == "post":
            body = operation.get("requestBody", {}).get("content", {}).get("application/json", {})
            if body.get("schema", {}).get("$ref") != "../schemas/register-article-v1.json":
                raise AssertionError(f"Article v1 client lost its JSON request for POST {path}")
        response = operation.get("responses", {}).get(expected["status"], {})
        schema = response.get("content", {}).get("application/json", {}).get("schema", {})
        if schema.get("$ref") != "../schemas/article-v1.json":
            raise AssertionError(f"Article v1 reader lost its JSON response for {method.upper()} {path}")

    guaranteed = set(response_schema.get("required", [])) & response_schema.get("properties", {}).keys()
    missing = previous["response"].keys() - guaranteed
    if missing:
        raise AssertionError(f"Article v1 reader lost guaranteed fields: {', '.join(sorted(missing))}")
    path = ROOT / "schemas/article-v1.json"
    current = registry.with_resource(path.as_uri(), Resource.from_contents(response_schema))
    Draft202012Validator({"$ref": path.as_uri()}, registry=current,
                        format_checker=FormatChecker()).validate(previous["response"])


validate_url(OPENAPI.as_uri())
validate_url((ROOT / "openapi/manufacturing-v1.yaml").as_uri())
manifest_path = ROOT / "manifests/master-data-v1.json"
manifest = load(manifest_path)
validator("service-manifest-v1.json").validate(manifest)
message = load(ROOT / "examples/nats/article-registered-v1.json")
route, = manifest["messaging"]
wire_schema = (manifest_path.parent / route["contract"]).resolve()
assert wire_schema in schemas
wire = validator(wire_schema.name)
body = message["envelope"]
wire.validate(body)
assert message["subject"] == body["messageType"] == route["subject"].format(tenantId=body["tenantId"])
for field, header in (("protocolVersion", "Protocol-Version"), ("schemaVersion", "Schema-Version"),
                      ("tenantId", "Tenant-Id"), ("organizationId", "Organization-Id"),
                      ("serviceVersion", "Service-Version"), ("messageId", "Message-Id")):
    assert message["headers"][f"Erp4us-{header}"] == body[field]
assert wire.is_valid({**body, "payload": {**body["payload"], "optionalExtra": True}})
assert not wire.is_valid({**body, "schemaVersion": "2.0.0"})
assert not wire.is_valid({**body, "protocolVersion": "2.0.0"})
assert not wire.is_valid({**body, "payload": {"revision": 1}})

config_path = ROOT / "schemas/nats-relay-heartbeat-config-v1.json"
config = load(ROOT / "examples/nats/relay-heartbeat-config-v1.json")
for kind, example in config.items():
    contract = Draft202012Validator({"$ref": f"{config_path.as_uri()}#/$defs/{kind}"},
                                    registry=registry, format_checker=FormatChecker())
    contract.validate(example)
    if kind in ("query", "changed"):
        assert not contract.is_valid({**example, "tenantId": body["tenantId"], "actor": body["actor"]})
        assert not contract.is_valid({**example, "schemaVersion": "2.0.0"})
        assert not contract.is_valid({**example, "protocolVersion": "2.0.0"})
    if kind == "changed":
        assert not contract.is_valid({**example, "payload": {"revision": 0}})
    if kind == "reply":
        for period in (-1, 3601, "30"):
            assert not contract.is_valid({"result": {"revision": 1, "heartbeatSeconds": period}})

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
previous_article = load(Path(__file__).with_name("fixtures") / "article-reader-v1.json")
command.validate(previous_article["request"])
for bad in ({"sku": "A\n", "name": "Name"}, {"sku": "A", "name": "\u0085"},
            {"sku": "A", "name": " ", "tenant_id": str(uuid4())}):
    assert not command.is_valid(bad)
article_schema = schemas[ROOT / "schemas/article-v1.json"]
check_article_reader(api, article_schema, previous_article)

additive = deepcopy(article_schema)
additive["properties"]["description"] = {"type": "string"}
check_article_reader(api, additive, previous_article)

breaking_api = deepcopy(api)
del breaking_api["paths"]["/v1/articles/{id}"]["get"]
breaking_schema = deepcopy(article_schema)
breaking_schema["required"].remove("name")
for candidate_api, candidate_schema, expected_error in (
    (breaking_api, article_schema, "lost GET /v1/articles/{id}"),
    (api, breaking_schema, "lost guaranteed fields: name"),
):
    try:
        check_article_reader(candidate_api, candidate_schema, previous_article)
    except AssertionError as error:
        assert expected_error in str(error)
    else:
        raise AssertionError(f"Compatibility check missed regression: {expected_error}")
envelope = validator("event-envelope-v1.json")
event = {
    "messageId": str(uuid4()), "messageType": "business.sales.order.confirmed.v1",
    "schemaVersion": 1, "timestamp": "2026-11-23T12:00:00Z", "tenantId": str(uuid4()),
    "actor": {"kind": "service", "id": "erp4us.sales"}, "correlationId": str(uuid4()),
    "sourceService": "erp4us.sales", "payload": {"orderId": str(uuid4())},
}
envelope.validate(event)
assert not envelope.is_valid({**event, "tenantId": "not-a-uuid"})
registered = validator("article-registered-v1.json")
article_event = {
    **event, "messageType": "business.master-data.article.registered.v1",
    "sourceService": "erp4us.master-data", "organizationId": str(uuid4()),
    "payload": {"articleId": str(uuid4()), "revision": 1},
}
registered.validate(article_event)
assert not registered.is_valid({**article_event, "payload": {"articleId": str(uuid4())}})
assert not registered.is_valid({**article_event, "messageType": event["messageType"]})
shortage = load(ROOT / "fixtures" / "stock-shortage-v1.json")
shortage_validator = validator("stock-shortage-v1.json")
shortage_validator.validate(shortage)
shortage_validator.validate({**shortage, "missingQuantity": 0})
for invalid in ({"missingQuantity": -1}, {"missingQuantity": 2147483648},
                {"missingQuantity": 0.5}, {"unit": "kg"}, {"tenantId": str(uuid4())}):
    assert not shortage_validator.is_valid({**shortage, **invalid})
proposal_query = validator("get-draft-proposal-v1.json")
query = {"contract": "get-draft-proposal.v1", "shortageId": shortage["shortageId"]}
proposal_query.validate(query)
assert not proposal_query.is_valid({**query, "organizationId": str(uuid4())})

line = load(ROOT / "fixtures" / "confirmed-stock-line-v1.json")
confirmed = validator("confirmed-stock-line-v1.json")
confirmed.validate(line)
for invalid in ({"unit": "kg"}, {"quantity": 0}, {"quantity": 1.5}, {"tenantId": str(uuid4())}):
    assert not confirmed.is_valid({**line, **invalid})
cancel = load(ROOT / "fixtures" / "cancel-stock-line-v1.json")
cancellation = validator("cancel-stock-line-v1.json")
cancellation.validate(cancel)
for invalid in ({"quantity": 6}, {"organizationId": str(uuid4())}, {"contract": line["contract"]}):
    assert not cancellation.is_valid({**cancel, **invalid})
register_unit = validator("register-unit-v1.json")
register_unit.validate({"code": "EA"})
for invalid in ({"code": ""}, {"code": "kg"}, {"code": "EA\n"}, {"code": "1EA"},
                {"code": "A" * 17}, {"code": "EA", "tenant_id": str(uuid4())}):
    assert not register_unit.is_valid(invalid)
validator("unit-v1.json").validate({
    "id": str(uuid4()), "tenant_id": str(uuid4()), "organization_id": str(uuid4()),
    "created_by": str(uuid4()), "schema_version": 1, "revision": 1, "code": "EA",
})
print("OpenAPI, JSON Schemas and protocol examples validated")
