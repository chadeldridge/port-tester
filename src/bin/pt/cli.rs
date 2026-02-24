use hostname_validator::is_valid as is_valid_hostname;
use log::debug;
use pt::Verbosity;
use std::net::IpAddr;

use clap::{ArgAction, CommandFactory, Parser, value_parser};

const DEFAULT_COUNT: u32 = 0;
const DEFAULT_INTERVAL: u64 = 1;
const DEFAULT_PORT: u16 = 443;
const DEFAULT_TIMEOUT: u64 = 5;

#[macro_export]
macro_rules! count_true_u8 {
    () => (0 as u8);
    ($elem:expr; $n:expr) => (
        let v = vec![$elem];
        _cout_true(v)
    );
    ($($x:expr),+$(,)?) => (
        {
            let v = vec![$($x),+];
            _count_true(v)
        }
    );
}

fn _count_true(vec: std::vec::Vec<bool>) -> usize {
    vec.into_iter().filter(|&b| b).count()
}

#[derive(Debug, Default, Parser)]
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
    /// Interval to output intermediate reports.
    /// Default is 0 (no intermediate reports).
    /// If set to N, a report will be printed every N attempts.
    #[arg(short, long, default_value_t = 0)]
    pub report_interval: u32,
    /// Connection attempt timeout in seconds.
    #[arg(short, long, default_value_t = DEFAULT_TIMEOUT)]
    pub timeout: u64,
    //#[arg(long, action = clap::ArgAction::Help, help = "Print help information")]
    //pub help: Option<bool>,
    /// Quiet mode.
    /// Suppress per-attempt output and attempt errors only showing sequence numbers and each result
    /// as 'ok' or 'failed'.
    #[arg(short, long, default_value_t = false)]
    pub quiet: bool,
    /// Silent mode.
    /// Suppress output except for errors and final report.
    #[arg(short, long, default_value_t = false)]
    pub silent: bool,
    /// Verbose level.
    /// Default to 1.
    /// 1 = warnings
    /// 2 = debug
    /// 3 = trace
    #[arg(short, long, action = ArgAction::Count, default_value_t = 0)]
    pub verbose: u8,
}

/*
impl Default for Args {
    fn default() -> Self {
        Args::parse()
    }
}
*/

impl Args {
    pub fn new() -> Self {
        Args::parse()
    }
}

#[derive(Debug, Default)]
pub struct Cli {
    pub args: Args,
    pub verbose: Option<Verbosity>,
}

impl Cli {
    pub fn new(args: Args) -> Self {
        debug!("Initializing CLI");
        let mut c = Cli {
            args,
            verbose: None,
        };

        let is_verbose = !matches!(&c.args.verbose, 0);

        // Print help and exit if conflicting arguments are given.
        if count_true_u8!(c.args.silent, c.args.quiet, is_verbose) > 1 {
            eprintln!("You may only specify one of: --quiet, --silent, --verbose");
            let _ = Args::command().print_help();
            std::process::exit(3);
        }

        // Set verbosity so we know how much to print.
        c.verbose = c.verbosity();
        c
    }

    pub fn print_help() {
        let _ = Args::command().print_help();
    }

    pub fn verbosity(&self) -> Option<Verbosity> {
        if self.args.quiet {
            Some(Verbosity::Quiet)
        } else if self.args.silent {
            Some(Verbosity::Silent)
        } else if self.args.verbose > 0 {
            Some(Verbosity::Verbose(self.args.verbose))
        } else {
            None
        }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_count_true() {
        let v = vec![true, false, true];
        assert_eq!(_count_true(v), 2);
    }

    #[test]
    fn test_count_true_u8() {
        let c = count_true_u8!(true, false, true);
        assert_eq!(c, 2);
    }

    #[test]
    fn test_validate_host() {
        let empty = "";
        let hostname = "example.com";
        let ip = "1.1.1.1";
        let invalid = "-a.com";

        assert!(validate_host(empty).is_err());
        assert!(validate_host(hostname).is_ok());
        assert!(validate_host(ip).is_ok());
        assert!(validate_host(invalid).is_err());
    }

    #[test]
    fn test_cli_new() {
        let mut args = Args::try_parse_from(vec!["pt", "1.1.1.1"]);
        assert!(args.is_ok());
        let mut cli = Cli::new(args.unwrap());
        assert_eq!(cli.args.host, "1.1.1.1".to_string());

        args = Args::try_parse_from(vec!["pt", "1.1.1.1", "--silent"]);
        assert!(args.is_ok());
        cli = Cli::new(args.unwrap());
        assert_eq!(cli.verbose.unwrap(), Verbosity::Silent);
    }

    #[test]
    fn test_verbosity() {
        let mut args = Args::try_parse_from(vec!["pt", "1.1.1.1"]);
        assert!(args.is_ok());
        let mut cli = Cli::new(args.unwrap());
        assert_eq!(cli.verbose, None);

        args = Args::try_parse_from(vec!["pt", "1.1.1.1", "--silent"]);
        assert!(args.is_ok());
        cli = Cli::new(args.unwrap());
        assert_eq!(cli.verbose.unwrap(), Verbosity::Silent);

        args = Args::try_parse_from(vec!["pt", "1.1.1.1", "--quiet"]);
        assert!(args.is_ok());
        cli = Cli::new(args.unwrap());
        assert_eq!(cli.verbose.unwrap(), Verbosity::Quiet);

        args = Args::try_parse_from(vec!["pt", "1.1.1.1", "-v"]);
        assert!(args.is_ok());
        cli = Cli::new(args.unwrap());
        assert_eq!(cli.verbose.unwrap(), Verbosity::Verbose(1));

        args = Args::try_parse_from(vec!["pt", "1.1.1.1", "-vvv"]);
        assert!(args.is_ok());
        cli = Cli::new(args.unwrap());
        assert_eq!(cli.verbose.unwrap(), Verbosity::Verbose(3));
    }
}
