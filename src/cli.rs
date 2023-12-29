use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser, Serialize)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(
        long,
        value_name = "port_sink",
        help = "Port to expose sink server on locally"
    )]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub port_sink: Option<u16>,
    #[arg(
        long,
        value_name = "port_dashboard",
        help = "Port to expose admin dashboard server on locally"
    )]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub port_dashboard: Option<u16>,
    #[arg(
        long,
        value_name = "requests_folder",
        help = "Folder to use (and create if not present) to save received requests"
    )]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub requests_folder: Option<String>,
    // #[command(subcommand)]
    // command: Option<Commands>,
}
