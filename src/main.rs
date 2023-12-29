use tokio;

use tracing::Level;

use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod config;
mod dashboard;
mod sink;
mod templates;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .compact();

    let filter_layer = filter::Targets::new()
        .with_target("tower_http::trace::on_response", Level::DEBUG)
        // .with_target("tower_http::trace::on_request", Level::DEBUG)
        .with_target("tower_http::trace::make_span", Level::DEBUG)
        .with_default(Level::INFO);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    let settings = &config::SETTINGS;

    let sink = sink::run(settings.port_sink);
    let dashbaord = dashboard::run(settings.port_dashboard);

    let (_sink_server, _dashboard_server) = tokio::join!(sink, dashbaord);

    Ok(())
}
