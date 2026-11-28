//! Shared setup for the two ignored receipt transport tests, not application configuration.

use erp4us_messaging::{ConnectOptions, Connection, JetStream};
use mongodb::{
    Client, Database,
    options::{ClientOptions, ServerAddress},
};
use std::{num::NonZeroU16, time::Duration};
use uuid::Uuid;

pub(super) const STREAM_MARKER: &str = "Isolated ERP4US receipt transport proof";
pub(super) const WAIT: Duration = Duration::from_secs(5);

pub(super) async fn connect(owner: &str) -> (Database, JetStream, bool) {
    fn port(name: &str) -> NonZeroU16 {
        std::env::var(name)
            .ok()
            .and_then(|value| {
                let port: NonZeroU16 = value.parse().ok()?;
                (port.to_string() == value).then_some(port)
            })
            .unwrap_or_else(|| panic!("{name} must name a disposable loopback port"))
    }

    let nats = port("RECEIPT_TRANSPORT_TEST_NATS_PORT");
    let prefix = format!("{owner}_receipt_wire_");
    let (db, drop_database) = if let Ok(uri) = std::env::var("RECEIPT_TRANSPORT_TEST_MONGO_URI") {
        // The managed fixture may use only the authenticated, disposable
        // loopback database allocated specifically for this owner and test.
        assert!(uri.starts_with("mongodb://fixture:"));
        let options = ClientOptions::parse(uri)
            .await
            .expect("valid managed receipt test Mongo settings");
        assert!(
            matches!(&options.hosts[..], [ServerAddress::Tcp {host, port: Some(_)}] if host == "127.0.0.1")
        );
        assert_eq!(options.direct_connection, Some(true));
        let database = options.default_database.as_deref().expect("test database");
        let suffix = database
            .strip_prefix(&prefix)
            .expect("owner test namespace");
        let uuid = Uuid::try_parse(suffix).expect("generated test database");
        assert!(uuid.get_version_num() == 4 && uuid.simple().to_string() == suffix);
        let db = Client::with_options(options)
            .expect("managed test client")
            .default_database()
            .expect("managed test database");
        (db, false)
    } else {
        let mongo = port("RECEIPT_TRANSPORT_TEST_MONGO_PORT");
        let db = Client::with_uri_str(format!(
            "mongodb://127.0.0.1:{mongo}/?directConnection=true&serverSelectionTimeoutMS=3000"
        ))
        .await
        .expect("local receipt test Mongo settings")
        .database(&format!("{prefix}{}", Uuid::new_v4().simple()));
        (db, true)
    };
    let broker = Connection::connect(
        &format!("nats://127.0.0.1:{nats}"),
        ConnectOptions::new()
            .connection_timeout(WAIT)
            .max_reconnects(1)
            .ignore_discovered_servers(),
    )
    .await
    .expect("disposable local receipt test NATS");
    (db, broker.jetstream(WAIT), drop_database)
}
