use hostname_validator::is_valid as is_valid_hostname;
use std::net::IpAddr;

use clap::{CommandFactory, Parser, value_parser};

const DEFAULT_COUNT: u32 = 0;
const DEFAULT_INTERVAL: u64 = 1;
const DEFAULT_PORT: u16 = 443;
const DEFAULT_TIMEOUT: u64 = 5;

#[derive(Debug, Parser)]
//#[command(disable_help_flag = true, version, about, long_about = None)]
#[command(version, about, long_about = None)]
pub struct Args {
    // Positional Arguments
    /// Target host to connect to.
    #[arg(value_parser = validate_host)]
    pub host: String,
    /// Port number to connect to.
    #[arg(value_parser = value_parser!(u16).range(1..), default_value_t = DEFAULT_PORT)]
    pub port: u16,

    // Options
    /// Count of connection attempts.
    /// Use 0 for infinite attempts.
    #[arg(short, long, default_value_t = DEFAULT_COUNT)]
    pub count: u32,
    /// Interval between attempts in seconds.
    #[arg(short, long, default_value_t = DEFAULT_INTERVAL)]
    pub interval: u64,
    /// Quiet mode.
    /// Suppress per-attempt output and attempt errors only showing sequence numbers and each result
    /// as 'ok' or 'failed'.
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,
    /// Interval to output intermediate reports.
    /// Default is 0 (no intermediate reports).
    /// If set to N, a report will be printed every N attempts.
    #[arg(short, long, default_value_t = 0)]
    pub report_interval: u32,
    /// Silent mode.
    /// Suppress output except for errors and final report.
    #[arg(short, long, default_value_t = false)]
    pub silent: bool,
    /// Connection attempt timeout in seconds.
    #[arg(short, long, default_value_t = DEFAULT_TIMEOUT)]
    pub timeout: u64,
    //#[arg(long, action = clap::ArgAction::Help, help = "Print help information")]
    //pub help: Option<bool>,
}

impl Default for Args {
    fn default() -> Self {
        Args::parse()
    }
}

impl Args {
    pub fn print_help() {
        let _ = Args::command().print_help();
    }
}

fn validate_host(host: &str) -> Result<String, String> {
    if host.trim().is_empty() {
        Err(String::from("Host cannot be empty"))
    } else if host.parse::<IpAddr>().is_ok() || is_valid_hostname(host) {
        Ok(host.to_string())
    } else {
        Err(String::from("Invalid host format"))
    }
}
