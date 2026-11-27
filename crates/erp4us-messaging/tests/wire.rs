use erp4us_messaging::{
    Contract, Delivery, Envelope, Error, HeaderMap, Kind, Layer, Route, Scope, Subject,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArticleRegistered {
    article_id: Uuid,
    revision: u32,
}

impl Contract for ArticleRegistered {
    const ROUTE: Route<'static> = Route {
        layer: Layer::Business,
        domain: "master-data",
        kind: Kind::Event,
        resource: "article",
        action: "registered",
    };
    const SCHEMA_VERSION: &'static str = "1.0.0";
    const PROTOCOL_SUPPORT: &'static str = "=1.0.0";
    const SCHEMA_SUPPORT: &'static str = "=1.0.0";
    const DELIVERY: Delivery = Delivery::JetStream("ERP4US_BUSINESS_EVENTS_V1");
}

fn fixture() -> (String, HeaderMap, Value) {
    let mut data: Value = serde_json::from_str(include_str!(
        "../../../contracts/examples/nats/article-registered-v1.json"
    ))
    .unwrap();
    let raw = data["headers"].as_object().unwrap();
    let mut headers = HeaderMap::new();
    for (name, value) in raw {
        headers.insert(name.as_str(), value.as_str().unwrap());
    }
    (
        data["subject"].as_str().unwrap().to_owned(),
        headers,
        data["envelope"].take(),
    )
}

#[test]
fn shared_fixture_and_additive_fields_decode() {
    let (subject, headers, mut body) = fixture();
    let decode = |body: &Value| {
        Envelope::<ArticleRegistered>::decode(
            &subject,
            &headers,
            &serde_json::to_vec(body).unwrap(),
        )
    };
    let envelope = decode(&body).unwrap();
    assert_eq!(envelope.subject().unwrap(), subject);
    assert_eq!(envelope.payload.revision, 1);
    body["payload"]["optionalFutureField"] = json!(true);
    assert!(decode(&body).is_ok());
    body["payload"].as_object_mut().unwrap().remove("articleId");
    assert!(matches!(decode(&body), Err(Error::Deserialize(_))));
}

#[test]
fn versions_are_independent_and_noncanonical_subjects_fail() {
    let route = ArticleRegistered::ROUTE;
    for (protocol, schema) in [(1, 1), (1, 2), (2, 1)] {
        let subject = route.subject(protocol, Scope::System, schema).unwrap();
        let parsed: Subject = subject.parse().unwrap();
        assert_eq!(
            (parsed.protocol_major, parsed.schema_major),
            (protocol, schema)
        );
    }
    let (subject, _, _) = fixture();
    for bad in [
        subject[3..].to_string(),
        subject.replace(".v1", ""),
        subject.replacen("v1.", "v01.", 1),
        subject.replace(".v1", ".v1.0"),
        subject.replace(".business.", ".production."),
        subject.replace(".master-data.", ".*."),
        subject.replace(".ddcae139-c1e8-47d2-8b48-349cb6f30ce5.", ".tenant01."),
    ] {
        assert!(bad.parse::<Subject>().is_err(), "{bad}");
    }
    assert!(route.subject(1, Scope::Tenant(Uuid::nil()), 1).is_err());
    assert_eq!(
        route.all_scopes(1, 2).unwrap(),
        "v1.business.*.master-data.event.article.registered.v2"
    );
}

#[test]
fn refuses_unreviewed_versions_and_ambiguous_metadata() {
    let (subject, mut headers, mut body) = fixture();
    let decode = |headers: &HeaderMap, body: &Value| {
        Envelope::<ArticleRegistered>::decode(&subject, headers, &serde_json::to_vec(body).unwrap())
    };
    body["protocolVersion"] = json!("1.1.0");
    assert!(matches!(
        decode(&headers, &body),
        Err(Error::UnsupportedProtocol)
    ));
    body["protocolVersion"] = json!("1.0.0");
    body["schemaVersion"] = json!("1.1.0");
    assert!(matches!(
        decode(&headers, &body),
        Err(Error::UnsupportedSchema)
    ));
    body["schemaVersion"] = json!("1.0.0");
    body["tenantId"] = json!("87ae5f2a-c155-405a-b62a-5b6fe238db33");
    assert!(matches!(
        decode(&headers, &body),
        Err(Error::InvalidEnvelope(_))
    ));
    let (_, _, body) = fixture();
    for key in [
        "Erp4us-Schema-Version",
        "Erp4us-Protocol-Version",
        "Erp4us-Message-Id",
        "Erp4us-Tenant-Id",
        "Erp4us-Organization-Id",
        "Nats-Msg-Id",
    ] {
        let mut bad = headers.clone();
        bad.insert(key, "mismatch");
        assert!(matches!(decode(&bad, &body), Err(Error::InvalidHeader(actual)) if actual == key));
    }
    headers.append("erp4us-tenant-id", body["tenantId"].as_str().unwrap());
    assert!(matches!(
        decode(&headers, &body),
        Err(Error::InvalidHeader("Erp4us-Tenant-Id"))
    ));
    assert!(matches!(
        decode(&HeaderMap::new(), &body),
        Err(Error::InvalidHeader(_))
    ));
}
