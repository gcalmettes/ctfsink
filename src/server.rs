use axum::{
    body::Body,
    middleware::{self},
    routing::{any, get},
    Router,
};
use std::net::{Ipv4Addr, SocketAddr};
use tokio;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

use crate::{db, handlers};
use tokio::signal;

pub async fn run_sink(port: u16) {
    let localhost_v4 = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port);
    let listener_v4 = TcpListener::bind(&localhost_v4).await.unwrap();
    let db = db::Db::new().await;

    let app = Router::new()
        .route("/", any(handlers::sink::root))
        .route("/{*anyroute}", any(handlers::sink::any_path))
        .layer(TraceLayer::new_for_http())
        .with_state(db);

    tracing::info!("Sink listening on {}", localhost_v4);
    axum::serve(listener_v4, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal("sink"))
        .await
        .unwrap();
}

pub async fn run_dashboard(port: u16) {
    let localhost_v4 = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port);
    let listener_v4 = TcpListener::bind(&localhost_v4).await.unwrap();
    let db = db::Db::new().await;

    let app = Router::new()
        .route("/", get(handlers::dashboard::home))
        .route("/detail/{encoded}", get(handlers::dashboard::detail))
        .route("/static/{*file}", get(handlers::dashboard::static_handler))
        .with_state(db);

    tracing::info!("Dashboard available at {}", localhost_v4);
    axum::serve(listener_v4, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal("dashboard"))
        .await
        .unwrap();
}

async fn shutdown_signal(name: &str) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            println!("shutting down {} following SIGINT", name)
        },
        _ = terminate => {
            println!("shutting down {} following SIGTERM", name)
        },
    }
}

pub async fn run_proxy(port: u16, proxy_url: &str) {
    let localhost_v4 = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port);
    let listener_v4 = TcpListener::bind(&localhost_v4).await.unwrap();
    let db = db::Db::new().await;
    let client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpConnector::new());

    let proxy_state = handlers::proxy::ProxyState {
        client: client.clone(),
        url_to: proxy_url.to_string(),
    };

    let app = Router::new()
        .route("/{*anyroute}", any(handlers::proxy::proxy))
        .layer(TraceLayer::new_for_http())
        // .layer(TraceLayer::new_for_http().make_span_with(tracing::info_span!("service", "proxy")))
        .layer(middleware::from_fn_with_state(
            db,
            handlers::proxy::save_request_and_response,
        ))
        .with_state(proxy_state);

    tracing::info!(
        "Proxy listening on {} and proxying {}",
        localhost_v4,
        proxy_url
    );
    axum::serve(listener_v4, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal("proxy"))
        .await
        .unwrap();
}
