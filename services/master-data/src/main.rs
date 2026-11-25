mod domain;
mod http;
mod partners;
mod store;

use std::{env, io, net::SocketAddr};
use tokio::{net::TcpListener, signal};

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt().json().init();
    if env::var("ERP4US_MODE").as_deref() != Ok("local-demo") {
        return Err(io::Error::other(
            "Only explicit local-demo mode is supported",
        ));
    }
    let token = env::var("ERP4US_DEMO_TOKEN")
        .ok()
        .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| io::Error::other("ERP4US_DEMO_TOKEN must be 64 hex characters"))?;
    let uri = env::var("MASTER_DATA_MONGODB_URI")
        .map_err(|_| io::Error::other("MASTER_DATA_MONGODB_URI is required"))?;
    let store = store::Store::open(&uri)
        .await
        .map_err(|_| io::Error::other("Could not initialize the master-data database"))?;
    let address = env::var("ERP4US_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let address: SocketAddr = address
        .parse()
        .map_err(|_| io::Error::other("ERP4US_BIND must be a socket address"))?;
    let listener = TcpListener::bind(address).await?;
    tracing::info!(%address, "Master Data listening in local-demo mode; not for production");
    axum::serve(listener, http::router(store, token))
        .with_graceful_shutdown(shutdown())
        .await
}

async fn shutdown() {
    #[cfg(unix)]
    {
        use signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    signal::ctrl_c().await.expect("install interrupt handler");
}
