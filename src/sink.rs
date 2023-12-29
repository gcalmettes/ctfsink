use axum::{routing::any, Router};
use std::net::{Ipv4Addr, SocketAddr};
use tokio;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

mod handlers;
pub mod models;
mod utils;

pub async fn run(port: u16) {
    let localhost_v4 = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    let listener_v4 = TcpListener::bind(&localhost_v4).await.unwrap();

    let app = Router::new()
        .route("/", any(handlers::black_hole))
        .layer(TraceLayer::new_for_http());

    tracing::info!("Sink listening on {}", localhost_v4);
    axum::serve(listener_v4, app.into_make_service())
        .await
        .unwrap();
}
