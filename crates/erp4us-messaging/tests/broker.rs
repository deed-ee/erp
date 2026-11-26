use erp4us_messaging::{
    ConnectOptions, Connection, Contract, Delivery, Envelope, Error, Kind, Layer, Route, Scope,
    Version,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Deserialize, Serialize)]
struct Settings<const PROTOCOL: u64, const SCHEMA: u64> {
    key: String,
}

impl<const PROTOCOL: u64, const SCHEMA: u64> Contract for Settings<PROTOCOL, SCHEMA> {
    const ROUTE: Route<'static> = Route {
        layer: Layer::Config,
        domain: "messaging-test",
        kind: Kind::Query,
        resource: "settings",
        action: "get",
    };
    const SCHEMA_VERSION: &'static str = if SCHEMA == 1 { "1.0.0" } else { "2.0.0" };
    const PROTOCOL_SUPPORT: &'static str = if PROTOCOL == 1 { "=1.0.0" } else { "=2.0.0" };
    const SCHEMA_SUPPORT: &'static str = Self::SCHEMA_VERSION;
    const DELIVERY: Delivery = Delivery::Core;
}

#[tokio::test]
#[ignore = "requires the isolated NATS container from tests/integration/messaging.sh"]
async fn request_reply_and_version_routes_on_real_broker() {
    let url = std::env::var("NATS_TEST_URL").expect("NATS_TEST_URL");
    let connection = Connection::connect(&url, ConnectOptions::new())
        .await
        .unwrap();
    let mut v1 = connection
        .subscribe::<Settings<1, 1>>(1, Scope::System, 1)
        .await
        .unwrap();
    let mut schema2 = connection
        .subscribe::<Settings<1, 2>>(1, Scope::System, 2)
        .await
        .unwrap();
    let mut protocol2 = connection
        .subscribe::<Settings<2, 1>>(2, Scope::System, 1)
        .await
        .unwrap();
    round_trip::<1, 1>(&connection, &mut v1).await;
    assert!(
        tokio::time::timeout(Duration::from_millis(100), schema2.next())
            .await
            .is_err()
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), protocol2.next())
            .await
            .is_err()
    );
    round_trip::<1, 2>(&connection, &mut schema2).await;
    round_trip::<2, 1>(&connection, &mut protocol2).await;
    v1.unsubscribe().await.unwrap();
    assert!(matches!(
        connection
            .request(&message::<1, 1>(), Duration::from_secs(1))
            .await,
        Err(Error::Request(_))
    ));
    connection.drain().await.unwrap();
}

fn message<const PROTOCOL: u64, const SCHEMA: u64>() -> Envelope<Settings<PROTOCOL, SCHEMA>> {
    Envelope::system(
        Settings {
            key: "synthetic".into(),
        },
        Version::new(PROTOCOL, 0, 0),
        "erp4us.messaging-test",
        Version::new(0, 1, 0),
    )
    .unwrap()
}

async fn round_trip<const PROTOCOL: u64, const SCHEMA: u64>(
    connection: &Connection,
    subscription: &mut erp4us_messaging::Subscriber,
) {
    let request = message::<PROTOCOL, SCHEMA>();
    let reply = json!({"result": {"revision": 1}});
    let serve = async {
        let message = tokio::time::timeout(Duration::from_secs(3), subscription.next())
            .await
            .unwrap()
            .unwrap();
        let envelope = Envelope::<Settings<PROTOCOL, SCHEMA>>::decode(
            message.subject.as_str(),
            message.headers.as_ref().unwrap(),
            &message.payload,
        )
        .unwrap();
        assert_eq!(envelope.payload.key, "synthetic");
        connection.reply(&message, &reply).await.unwrap();
    };
    let (_, response) = tokio::join!(serve, connection.request(&request, Duration::from_secs(3)));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&response.unwrap().payload).unwrap(),
        reply
    );
    assert!(matches!(
        connection.publish(&request).await,
        Err(Error::WrongTransport)
    ));
}
