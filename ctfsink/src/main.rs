use clap::{Parser, Subcommand};
use std::{env::var, fs::File, net::SocketAddr, path::Path};

use axum::{extract::ConnectInfo, response::Html, routing::get, Router};
use ngrok::prelude::*;

use serde::{Deserialize, Serialize};
use serde_yaml::{self};

#[derive(Debug, Serialize, Deserialize)]
struct NgrokConfig {
    version: String,
    authtoken: Option<String>,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(long, short, action, help = "Expose server through ngrok tunnel")]
    ngrok: bool,
    #[arg(
        short,
        long,
        value_name = "token",
        env = "NGROK_AUTHTOKEN",
        help = "Token to be used for ngrok, will read from ngrok config if not provided"
    )]
    token: Option<String>,
    #[arg(
        short,
        long,
        value_name = "port",
        env = "PORT",
        default_value = "5000",
        help = "Port to expose server on locally"
    )]
    port: u16,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Expose server through ngrok tunnel
    Ngrok,
}

// retrieve token from ngrok usual config path if no token provided.
fn get_ngrok_token(token: Option<String>) -> Option<String> {
    match token {
        Some(t) => Some(t),
        None => {
            // if ngrok config file is present, read token from it
            let mut ngrok_config_path = match var("HOME") {
                Ok(p) => p,
                _ => String::new(),
            };
            ngrok_config_path.push_str("/.config/ngrok/ngrok.yml");

            match Path::new(&ngrok_config_path).exists() {
                true => {
                    let f = File::open(ngrok_config_path).expect("Could not open file.");
                    let scrape_config: NgrokConfig =
                        serde_yaml::from_reader(f).expect("Could not read values.");
                    match scrape_config.authtoken {
                        Some(t) => Some(t),
                        None => None,
                    }
                }
                false => None,
            }
        }
    }
}

async fn handler(ConnectInfo(remote_addr): ConnectInfo<SocketAddr>) -> Html<String> {
    Html(format!("<h1>Hello, {remote_addr:?}!</h1>"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // build our application with a single route
    let app = Router::new().route("/", get(handler));

    let cli = Args::parse();

    // if ngrok subcommand or argument is present
    let start_ngrok = match &cli.command {
        Some(Commands::Ngrok) => true,
        None => cli.ngrok,
    };

    match start_ngrok {
        true => {
            // Instead of binding a local port like so:
            // axum::Server::bind(&"0.0.0.0:8000".parse().unwrap())
            // Run it with an ngrok tunnel instead!
            // Note that we still need a token to be able to start ngrok.
            match get_ngrok_token(cli.token) {
                Some(token) => {
                    let tun = ngrok::Session::builder()
                        .authtoken(token)
                        // Connect the ngrok session
                        .connect()
                        .await?
                        // Start a tunnel with an HTTP edge
                        .http_endpoint()
                        .listen()
                        .await?;

                    println!("Tunnel started on URL: {:?}", tun.url());

                    axum::Server::builder(tun)
                        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                        .await
                        .unwrap();
                }
                None => {
                    println!("Cannot start server behind ngrok as no ngrok token were found!");
                }
            }
        }
        false => {
            // Run local server.
            let addr = SocketAddr::from(([127, 0, 0, 1], cli.port));
            println!("listening on {}", addr);
            axum::Server::bind(&addr)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        }
    };

    Ok(())
}
