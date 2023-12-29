use axum::{routing::get, Router};
use std::net::{Ipv4Addr, SocketAddr};
use tokio;
use tokio::net::TcpListener;

mod handlers;

pub async fn run(port: u16) {
    let localhost_v4 = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    let listener_v4 = TcpListener::bind(&localhost_v4).await.unwrap();

    let app = Router::new().route("/", get(handlers::home));

    tracing::info!("Dashboard available at {}", localhost_v4);
    axum::serve(listener_v4, app.into_make_service())
        .await
        .unwrap();
}
