use pt::connectors::port_open::*;
use pt::core::error::*;
use pt::{Host, Verbosity};

use env_logger::Env;
use log::{debug, info};
use std::sync::Arc;

mod cli;

const DEFAULT_LOG_LEVEL: &str = "error";

fn main() {
    let cli = cli::Cli::new();
    setup_logger(&cli.verbose);

    let mut verbose = &Verbosity::default();
    if let Some(v) = &cli.verbose {
        verbose = v;
    }
    debug!("verbosity: {}", verbose);

    if cli.args.host.is_empty() {
        eprintln!("Error: Host is required.");
        cli::Cli::print_help();
        std::process::exit(1);
    }

    // Set up Ctrl-C handler to print report on interrupt. We need to create the host object first
    // so we can access its metrics in the handler.
    let host = match Host::new(&cli.args.host, cli.args.port) {
        Ok(h) => h,
        Err(e) => exit_handler(&e),
    };
    info!("host: {}", host.name());

    // Clone metrics so we have access to it in the ctrlc handler.
    let metrics_clone = Arc::clone(&host.metrics);
    // Create a handler that will attempt to print a metrics report when we receive a Ctrl-C.
    ctrlc::set_handler(move || {
        println!("\nInterrupted! Generating report...");
        metrics_clone.lock().unwrap().report();
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    // Get an iterator for the number of attempts. If count is 0, it will be infinite.
    let iter = if cli.args.count == 0 {
        debug!("attempts: infinite");
        std::iter::repeat(()).take(u32::MAX as usize)
    } else {
        let c = cli.args.count as usize;
        debug!("attempts: {}", c);
        std::iter::repeat(()).take(c)
    };

    for i in iter.enumerate().map(|(i, _)| i as u32 + 1) {
        debug!(
            "attempt: {}, ip: {}, port: {}, timeout: {}",
            i,
            host.ip(),
            host.port(),
            cli.args.timeout
        );

        // Do not report the attempt number if in silent mode or if we are only doing one attempt.
        if !cli.args.silent && cli.args.count != 1 {
            print!("{} ", i);
        }

        // Connect to the target and record metrics.
        handle_results(connect(&host, cli.args.timeout), verbose, cli.args.count);

        // Print intermediate report if report_interval is set.
        // If the count is reached, the final report will be printed after the loop.
        if cli.args.report_interval > 0
            && i % cli.args.report_interval == 0
            && (cli.args.count == 0 || i < cli.args.count)
        {
            print!("Intermediate report: ");
            host.metrics.lock().unwrap().report();
        }

        // Sleep between attempts unless this is the last attempt.
        if cli.args.count == 0 || i < cli.args.count {
            debug!("sleep: {}s", cli.args.interval);
            std::thread::sleep(std::time::Duration::from_secs(cli.args.interval));
        }
    }

    debug!("connection attempts complete, print final report");
    // Do not give the final report for a single attempt.
    if cli.args.count != 1 {
        host.metrics.lock().unwrap().report();
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

fn handle_results(result: Result<bool>, verbose: &Verbosity, count: u32) {
    match result {
        Ok(_) => {
            if !matches!(verbose, Verbosity::Silent) {
                println!("ok");
            } else if count == 1 {
                std::process::exit(0);
            }
        }
        Err(e) => {
            if !matches!(verbose, Verbosity::Silent) {
                if matches!(verbose, Verbosity::Quiet) {
                    println!("failed");
                } else {
                    println!("failed: {}", e);
                }
            } else if count == 1 {
                std::process::exit(1);
            }
        }
    }
}
