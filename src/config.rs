use crate::cli::Cli;
use clap::Parser;
use figment::{
    providers::{Env, Serialized},
    Figment,
};
use once_cell::sync::Lazy;
use serde::Deserialize;

pub static SETTINGS: Lazy<Settings> = Lazy::new(|| Settings::new());

// Settings are a singleton generated at runtime. All settings may be
// configured via environment variables. Example:
// PORT_SINK="xxx" would set the port for the sink webserver to the xxx value.
// Some settings are derived from other settings
#[derive(Deserialize, Debug)]
pub struct Settings {
    #[serde(default = "default_port_sink")]
    pub port_sink: u16,
    #[serde(default = "default_port_dashboard")]
    pub port_dashboard: u16,
    #[serde(default = "default_requests_folder")]
    pub requests_folder: String,
}

impl Settings {
    pub fn new() -> Self {
        Figment::new()
            .merge(Env::raw())
            .merge(Serialized::defaults(Cli::parse()))
            .extract()
            .unwrap()
    }
}

fn default_port_sink() -> u16 {
    5000
}

fn default_port_dashboard() -> u16 {
    5001
}

fn default_requests_folder() -> String {
    "00_requests".to_string()
}
