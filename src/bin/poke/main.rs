use cli::{Args, Cli};
use port_tester::connectors::port_open::*;
use port_tester::core::error::*;
use port_tester::{Host, Verbosity};

use env_logger::Env;
use log::{debug, info};

mod cli;

const DEFAULT_LOG_LEVEL: &str = "error";

fn main() {
    let cli = Cli::new(Args::new());
    setup_logger(&cli.verbose);

    let mut verbose = &Verbosity::default();
    if let Some(v) = &cli.verbose {
        verbose = v;
    }
    debug!("verbosity: {}", verbose);

    if cli.args.host.is_empty() {
        eprintln!("Error: Host is required.");
        Cli::print_help();
        std::process::exit(1);
    }

    let mut host = match Host::new(&cli.args.host, cli.args.port) {
        Ok(h) => h,
        Err(e) => exit_handler(&e),
    };
    info!("host: {}", host.name());

    // Create a handler that will attempt to print a metrics report when we receive a Ctrl-C.
    ctrlc::set_handler(move || {
        std::process::exit(1);
    })
    .expect("Error setting Ctrl-C handler");

    debug!(
        "attempt: {}, ip: {}, port: {}, timeout: {}",
        1,
        host.ip(),
        host.port(),
        cli.args.timeout
    );

    // Connect to the target and record metrics.
    connect(1, &mut host, cli.args.timeout);

    let status = host.metrics().result(1).unwrap().status();
    if !cli.args.silent {
        println!("{}", status.to_string_with_verbosity(verbose));
    }

    if status.is_err() {
        std::process::exit(1);
    } else {
        std::process::exit(0);
    }
}

fn setup_logger(level: &Option<Verbosity>) {
    // Prioritize log levels: cli flag > env var > default
    let env = Env::default()
        .filter_or("PT_LOG_LEVEL", DEFAULT_LOG_LEVEL)
        .write_style_or("PT_LOG_STYLE", "always");
    let mut builder = env_logger::Builder::from_env(env);
    if let Some(v) = level {
        builder.filter(None, v.to_filter_level());
    }
    builder.init();
}

/// Handle error and exit program.
fn exit_handler(error: &Error) -> ! {
    handle_error(error);
    if error.is_print_help() {
        cli::Cli::print_help();
    }
    debug!("Exiting with code {:?}", error.code());
    std::process::exit(error.code().unwrap_or(1));
}
