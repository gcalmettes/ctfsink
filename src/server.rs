use axum::{
    routing::{any, get},
    Router,
};
use std::net::{Ipv4Addr, SocketAddr};
use tokio;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use crate::{db, handlers};

pub async fn run_sink(port: u16) {
    let localhost_v4 = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    let listener_v4 = TcpListener::bind(&localhost_v4).await.unwrap();
    let db = db::Db::new().await;

    let app = Router::new()
        .route("/", any(handlers::sink::root))
        .route("/*anyroute", any(handlers::sink::any_path))
        .layer(TraceLayer::new_for_http())
        .with_state(db);

    tracing::info!("Sink listening on {}", localhost_v4);
    axum::serve(listener_v4, app.into_make_service())
        .await
        .unwrap();
}

pub async fn run_dashboard(port: u16) {
    let localhost_v4 = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    let listener_v4 = TcpListener::bind(&localhost_v4).await.unwrap();
    let db = db::Db::new().await;

    let app = Router::new()
        .route("/", get(handlers::dashboard::home))
        .route("/detail/:encoded", get(handlers::dashboard::detail))
        .route("/static/*file", get(handlers::dashboard::static_handler))
        .with_state(db);

    tracing::info!("Dashboard available at {}", localhost_v4);
    axum::serve(listener_v4, app.into_make_service())
        .await
        .unwrap();
}
