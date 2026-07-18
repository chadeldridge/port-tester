use log::debug;
use port_tester::Target;
use port_tester::Verbosity;
use port_tester::connectors::http::{HttpConfig, HttpSuccess};

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

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Parser)]
//#[command(disable_help_flag = true, version, about, long_about = None)]
#[command(version, about, long_about = None)]
pub struct Args {
    // Positional Arguments
    /// Target host to connect to.
    /// May be a bare hostname/IP or a URL including a scheme, port, and path
    /// (e.g. `https://example.com:8443/health`).
    pub host: String,
    /// Port number to connect to.
    /// Defaults to a port derived from the scheme (80/443) for HTTP tests, else 443.
    #[arg(value_parser = value_parser!(u16).range(1..))]
    pub port: Option<u16>,

    // Options
    /// Count of connection attempts to perform.
    /// 0 for infinite.
    #[arg(short, long, default_value_t = DEFAULT_COUNT)]
    pub count: u32,
    /// Perform an HTTP GET test instead of a plain port-open test.
    #[arg(long, default_value_t = false)]
    pub http: bool,
    /// Additional HTTP status code to accept as success. May be repeated.
    #[arg(long = "http-code", value_parser = value_parser!(u16).range(100..=599))]
    pub http_code: Vec<u16>,
    /// Restrict HTTP success to 2xx/3xx responses.
    #[arg(long, default_value_t = false)]
    pub http_success: bool,
    /// Perform an HTTP GET test over HTTPS regardless of port, scheme, or other indicators.
    #[arg(long, default_value_t = false)]
    pub https: bool,
    /// Allow insecure HTTPS: skip TLS certificate verification (expired/invalid certs,
    /// hostname mismatch when testing an IP, etc.). Like curl's --insecure.
    #[arg(short = 'k', long, default_value_t = false)]
    pub insecure: bool,
    /// Interval between attempts in seconds.
    #[arg(short, long, default_value_t = DEFAULT_INTERVAL)]
    pub interval: u64,
    /// Produce all output in JSON on exit. Output is held until all tests are complete.
    #[arg(long, conflicts_with_all = ["verbosity", "report_interval"], default_value_t = false)]
    pub json: bool,
    /// Quiet mode.
    /// Suppress per-attempt output and attempt errors only showing sequence numbers and each result
    /// as 'ok' or 'fail'.
    #[arg(short, long, group = "verbosity", default_value_t = false)]
    pub quiet: bool,
    /// Interval to output intermediate reports.
    /// Default is 0 (no intermediate reports).
    /// If set to N, a report will be printed every N attempts.
    #[arg(short, long, default_value_t = 0)]
    pub report_interval: u32,
    /// Silent mode.
    /// Suppress output except for errors and final report.
    #[arg(short, long, group = "verbosity", default_value_t = false)]
    pub silent: bool,
    /// Connection attempt timeout in seconds.
    #[arg(short, long, default_value_t = DEFAULT_TIMEOUT)]
    pub timeout: u64,
    /// Verbosity level.
    /// Defaults to 1.
    /// 1 = warnings
    /// 2 = debug
    /// 3 = trace
    #[arg(short, long, group = "verbosity", action = ArgAction::Count, default_value_t = 0)]
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

#[derive(Clone, Debug)]
pub struct Cli {
    pub args: Args,
    pub verbose: Option<Verbosity>,
    /// Bare host (hostname or IP) parsed from `args.host`.
    pub host: String,
    /// Resolved port to connect to.
    pub port: u16,
    /// HTTP test configuration, or [`None`] for a plain port-open test.
    pub http: Option<HttpConfig>,
}

impl Cli {
    pub fn new(args: Args) -> Self {
        debug!("Initializing CLI");
        let mut c = Cli {
            args,
            verbose: None,
            host: String::new(),
            port: DEFAULT_PORT,
            http: None,
        };

        let is_verbose = !matches!(&c.args.verbose, 0);

        // Print help and exit if conflicting arguments are given.
        if count_true_u8!(c.args.silent, c.args.quiet, is_verbose) > 1 {
            eprintln!("You may only specify one of: --quiet, --silent, --verbose");
            let _ = Args::command().print_help();
            std::process::exit(3);
        }

        // Parse the host argument into its scheme, host, port, and path components.
        let target = match Target::parse(&c.args.host) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(e.code().unwrap_or(3));
            }
        };

        // HTTP mode is on when requested by flag or implied by an explicit scheme prefix.
        let http_mode = c.args.http || c.args.https || target.scheme().is_some();

        // HTTP-only flags require an HTTP test.
        if !http_mode && (c.args.http_success || !c.args.http_code.is_empty() || c.args.insecure) {
            eprintln!("--http-success, --http-code, and --insecure require --http or --https");
            let _ = Args::command().print_help();
            std::process::exit(3);
        }

        c.host = target.host().to_string();

        if http_mode {
            let scheme = target.resolve_scheme(c.args.https, c.args.port);
            c.port = target.resolve_port(scheme, c.args.port);
            c.http = Some(HttpConfig {
                scheme,
                path: target.path().to_string(),
                success: HttpSuccess::from_flags(c.args.http_success, &c.args.http_code),
                timeout: c.args.timeout,
                insecure: c.args.insecure,
            });
        } else {
            c.port = target.port().or(c.args.port).unwrap_or(DEFAULT_PORT);
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

#[cfg(test)]
mod test {
    use super::*;
    use port_tester::Scheme;

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
    fn test_cli_new() {
        let mut args = Args::try_parse_from(vec!["pt", "1.1.1.1"]);
        assert!(args.is_ok());
        let mut cli = Cli::new(args.unwrap());
        assert_eq!(cli.args.host, "1.1.1.1".to_string());
        assert_eq!(cli.host, "1.1.1.1".to_string());

        args = Args::try_parse_from(vec!["pt", "1.1.1.1", "--silent"]);
        assert!(args.is_ok());
        cli = Cli::new(args.unwrap());
        assert_eq!(cli.verbose.unwrap(), Verbosity::Silent);
    }

    #[test]
    fn test_default_port_open_mode() {
        let cli = Cli::new(Args::try_parse_from(vec!["pt", "google.com"]).unwrap());
        assert!(cli.http.is_none());
        assert_eq!(cli.host, "google.com");
        assert_eq!(cli.port, 443);
    }

    #[test]
    fn test_port_open_embedded_port() {
        let cli = Cli::new(Args::try_parse_from(vec!["pt", "google.com:8080"]).unwrap());
        assert!(cli.http.is_none());
        assert_eq!(cli.port, 8080);
    }

    #[test]
    fn test_http_flag_defaults_http_scheme() {
        let cli = Cli::new(Args::try_parse_from(vec!["pt", "--http", "google.com"]).unwrap());
        let cfg = cli.http.expect("http mode");
        assert_eq!(cfg.scheme, Scheme::Http);
        assert_eq!(cli.port, 80);
        assert_eq!(cfg.path, "/");
        assert_eq!(cfg.success, HttpSuccess::Any);
    }

    #[test]
    fn test_https_flag_forces_tls() {
        let cli = Cli::new(Args::try_parse_from(vec!["pt", "--https", "google.com"]).unwrap());
        let cfg = cli.http.expect("http mode");
        assert_eq!(cfg.scheme, Scheme::Https);
        assert_eq!(cli.port, 443);
    }

    #[test]
    fn test_scheme_prefix_enables_http() {
        let cli = Cli::new(Args::try_parse_from(vec!["pt", "https://google.com/health"]).unwrap());
        let cfg = cli.http.expect("http mode");
        assert_eq!(cfg.scheme, Scheme::Https);
        assert_eq!(cli.host, "google.com");
        assert_eq!(cfg.path, "/health");
        assert_eq!(cli.port, 443);
    }

    #[test]
    fn test_http_embedded_port() {
        let cli = Cli::new(Args::try_parse_from(vec!["pt", "--http", "google.com:8443"]).unwrap());
        assert_eq!(cli.port, 8443);
    }

    #[test]
    fn test_http_success_policy() {
        let cli = Cli::new(
            Args::try_parse_from(vec![
                "pt",
                "--http",
                "--http-success",
                "--http-code",
                "418",
                "google.com",
            ])
            .unwrap(),
        );
        let cfg = cli.http.expect("http mode");
        assert!(cfg.success.accepts(200));
        assert!(cfg.success.accepts(418));
        assert!(!cfg.success.accepts(500));
    }

    #[test]
    fn test_insecure_flag() {
        let cli = Cli::new(Args::try_parse_from(vec!["pt", "--https", "-k", "1.1.1.1"]).unwrap());
        let cfg = cli.http.expect("http mode");
        assert!(cfg.insecure);

        // Without --insecure it defaults to secure.
        let cli = Cli::new(Args::try_parse_from(vec!["pt", "--https", "google.com"]).unwrap());
        assert!(!cli.http.expect("http mode").insecure);
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
